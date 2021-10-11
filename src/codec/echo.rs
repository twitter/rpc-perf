// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Keyspace;
use crate::*;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand_distr::Alphanumeric;

use std::borrow::Borrow;

pub struct Echo {
    config: Arc<Config>,
    rng: SmallRng,
}

impl Echo {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn echo(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut BytesMut) {
        let value = rng
            .sample_iter(&Alphanumeric)
            .take(keyspace.length())
            .collect::<Vec<u8>>();
        let crc = crc::crc32::checksum_ieee(&value);
        buf.extend_from_slice(&value);
        buf.put_u32(crc);
        buf.extend_from_slice(b"\r\n");
    }
}

impl Codec for Echo {
    fn encode(&mut self, buf: &mut BytesMut) {
        let keyspace = self.config.choose_keyspace(&mut self.rng);
        Self::echo(&mut self.rng, keyspace, buf)
    }

    fn decode(&self, buffer: &mut BytesMut) -> Result<(), ParseError> {
        // no-copy borrow as a slice
        let buf: &[u8] = (*buffer).borrow();

        // check if we got a CRLF
        let mut double_byte_windows = buf.windows(2);
        if let Some(response_end) = double_byte_windows.position(|w| w == b"\r\n") {
            if response_end < 5 {
                Err(ParseError::Unknown)
            } else {
                let message = &buf[0..(response_end - 4)];
                let crc = &buf[(response_end - 4)..response_end];
                let crc_calc = crc::crc32::checksum_ieee(message);
                let crc_bytes: [u8; 4] = unsafe { std::mem::transmute(crc_calc.to_be()) };
                if crc_bytes != crc[..] {
                    debug!("Response has bad CRC: {:?} != {:?}", crc, crc_bytes);
                    metrics::RESPONSE_EX.increment();
                    Err(ParseError::Error)
                } else {
                    let _ = buffer.split_to(response_end + 2);
                    Ok(())
                }
            }
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
