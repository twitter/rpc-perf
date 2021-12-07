// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::*;
use crate::config_file::Verb;
use crate::*;

use rand::rngs::SmallRng;
use rand::SeedableRng;

use std::borrow::Borrow;

pub struct Memcache {
    config: Arc<Config>,
    rng: SmallRng,
}

impl Memcache {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            rng: SmallRng::from_entropy(),
        }
    }

    fn get(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut BytesMut) {
        buf.extend_from_slice(b"get ");

        for i in 0..keyspace.batch_size() {
            let key = keyspace.generate_key(rng);
            buf.extend_from_slice(&key);
            if i + 1 < keyspace.batch_size() {
                buf.extend_from_slice(b" ");
            }
        }

        buf.extend_from_slice(b"\r\n");
    }

    fn set(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut BytesMut) {
        let key = keyspace.generate_key(rng);
        let value = keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec());
        let ttl = keyspace.ttl();
        buf.extend_from_slice(b"set ");
        buf.extend_from_slice(&key);
        buf.extend_from_slice(format!(" 0 {} {}\r\n", ttl, value.len()).as_bytes());
        buf.extend_from_slice(&value);
        buf.extend_from_slice(b"\r\n");
    }
}

impl Codec for Memcache {
    fn encode(&mut self, buf: &mut BytesMut) {
        let keyspace = self.config.choose_keyspace(&mut self.rng);
        let command = keyspace.choose_command(&mut self.rng);
        match command.verb() {
            Verb::Get => {
                metrics::REQUEST_GET.increment();
                Self::get(&mut self.rng, keyspace, buf)
            }
            Verb::Set => Self::set(&mut self.rng, keyspace, buf),
            _ => {
                unimplemented!()
            }
        }
    }

    fn decode(&self, buffer: &mut BytesMut) -> Result<(), ParseError> {
        // no-copy borrow as a slice
        let buf: &[u8] = (*buffer).borrow();

        for response in &[
            "STORED\r\n",
            "NOT_STORED\r\n",
            "EXISTS\r\n",
            "NOT_FOUND\r\n",
            "DELETED\r\n",
            "TOUCHED\r\n",
        ] {
            let bytes = response.as_bytes();
            if buf.len() >= bytes.len() && &buf[0..bytes.len()] == bytes {
                let _ = buffer.split_to(bytes.len());
                return Ok(());
            }
        }

        let mut windows = buf.windows(5);
        if let Some(response_end) = windows.position(|w| w == b"END\r\n") {
            let response = buffer.split_to(response_end + 5);
            let response_buf: &[u8] = response.borrow();
            let mut start = 0;
            let mut lines = response_buf.windows(2);
            while let Some(line_end) = lines.position(|w| w == b"\r\n") {
                if response_buf.len() >= 5 && &response_buf[start..(start + 5)] == b"VALUE" {
                    metrics::RESPONSE_HIT.increment();
                }
                start = line_end + 2;
            }
            return Ok(());
        }

        Err(ParseError::Incomplete)
    }
}
