// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::*;
use crate::config_file::Verb;
use crate::*;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand_distr::Alphanumeric;

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::io::{BufRead, BufReader};
use std::str;

pub struct PelikanRds {
    config: Arc<Config>,
    rng: SmallRng,
}

impl PelikanRds {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn get(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut BytesMut) {
        let key = rng
            .sample_iter(&Alphanumeric)
            .take(keyspace.length())
            .collect::<Vec<u8>>();
        buf.extend_from_slice(format!("*2\r\n$3\r\nget\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(&key);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn set(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut BytesMut) {
        let key = rng
            .sample_iter(&Alphanumeric)
            .take(keyspace.length())
            .collect::<Vec<u8>>();
        let value_len = keyspace.choose_value(rng).unwrap().length();
        let value = rng
            .sample_iter(&Alphanumeric)
            .take(value_len)
            .collect::<Vec<u8>>();
        let ttl = keyspace.ttl();

        if ttl != 0 {
            buf.extend_from_slice(b"*5\r\n");
        } else {
            buf.extend_from_slice(b"*3\r\n");
        }
        buf.extend_from_slice(format!("$3\r\nset\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(&key);
        buf.extend_from_slice(format!("\r\n${}\r\n", value.len()).as_bytes());
        buf.extend_from_slice(&value);
        buf.extend_from_slice(b"\r\n");
        if ttl != 0 {
            let formated_ttl = format!("{}", ttl);
            buf.extend_from_slice(b"$2\r\nEX\r\n");
            buf.extend_from_slice(format!("${}\r\n", formated_ttl.len()).as_bytes());
            buf.extend_from_slice(formated_ttl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }
}

impl Codec for PelikanRds {
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

        let end = &buf[buf.len() - 2..buf.len()];

        // All complete responses end in CRLF
        if end != b"\r\n" {
            return Err(ParseError::Incomplete);
        }

        let first_char = &buf[0..1];
        match str::from_utf8(first_char) {
            Ok("+") => {
                // simple string
                if buf.len() < 5 {
                    Err(ParseError::Incomplete)
                } else {
                    let msg = &buf[1..buf.len() - 2];
                    match str::from_utf8(msg) {
                        Ok("OK") | Ok("PONG") | Ok("NOOP") => {
                            let response_end = buf.len();
                            let _ = buffer.split_to(response_end);
                            Ok(())
                        }
                        _ => Err(ParseError::Unknown),
                    }
                }
            }
            Ok("-") => {
                // error response
                Err(ParseError::Error)
            }
            Ok(":") => {
                // numeric response
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(msg) {
                    Ok(msg) => match msg.parse::<i64>() {
                        Ok(_) => Ok(()),
                        Err(_) => Err(ParseError::Unknown),
                    },
                    Err(_) => Err(ParseError::Unknown),
                }
            }
            Ok("$") => {
                // bulk string
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(msg) {
                    Ok("-1") => {
                        let response_end = buf.len();
                        let _ = buffer.split_to(response_end);
                        Ok(())
                    }
                    Ok(_) => {
                        let reader = BufReader::new(buf);
                        let mut lines = reader.lines();
                        let mut line = lines.next().unwrap().unwrap();
                        let _ = line.remove(0);
                        match line.parse::<usize>() {
                            Ok(expected) => {
                                // data len = buf.len() - line.len() - 2x CRLF - 1
                                let have = buf.len() - line.len() - 5;
                                match have.cmp(&expected) {
                                    Ordering::Less => Err(ParseError::Incomplete),
                                    Ordering::Equal => {
                                        let response_end = buf.len();
                                        let _ = buffer.split_to(response_end);
                                        metrics::RESPONSE_HIT.increment();
                                        Ok(())
                                    }
                                    Ordering::Greater => Err(ParseError::Error),
                                }
                            }
                            Err(_) => Err(ParseError::Unknown),
                        }
                    }
                    Err(_) => Err(ParseError::Unknown),
                }
            }
            Ok("*") => {
                // arrays
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(msg) {
                    Ok("-1") => {
                        let response_end = buf.len();
                        let _ = buffer.split_to(response_end);
                        Ok(())
                    }
                    Ok(_) => {
                        // TODO: implement array parsing
                        Err(ParseError::Unknown)
                    }
                    Err(_) => Err(ParseError::Unknown),
                }
            }
            _ => Err(ParseError::Unknown),
        }
    }
}
