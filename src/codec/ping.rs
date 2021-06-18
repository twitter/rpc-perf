// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::*;

use std::borrow::Borrow;

pub struct Ping;

impl Ping {
    pub fn new(_config: Arc<Config>) -> Self {
        Self
    }

    pub fn ping(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(b"PING\r\n");
    }
}

impl Codec for Ping {
    fn encode(&mut self, buf: &mut BytesMut) {
        self.ping(buf)
    }

    fn decode(&self, buffer: &mut BytesMut) -> Result<(), ParseError> {
        // no-copy borrow as a slice
        let buf: &[u8] = (*buffer).borrow();

        // check if we got a CRLF
        let mut double_byte_windows = buf.windows(2);
        if let Some(response_end) = double_byte_windows.position(|w| w == b"\r\n") {
            match &buf[0..response_end] {
                b"pong" | b"PONG" => {
                    let _ = buffer.split_to(response_end + 2);
                    Ok(())
                }
                _ => Err(ParseError::Unknown),
            }
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
