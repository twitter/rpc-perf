// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;
use std::io::{BufRead, BufReader};

use bytes::{Buf, BytesMut};

pub struct Memcache {
    common: Common,
}

impl Memcache {
    pub fn new() -> Self {
        Self {
            common: Common::new(),
        }
    }

    pub fn get(&self, buf: &mut BytesMut, key: &[u8]) {
        buf.extend_from_slice(b"get ");
        buf.extend_from_slice(key);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn set(
        &self,
        buf: &mut BytesMut,
        key: &[u8],
        value: &[u8],
        exptime: Option<u32>,
        flags: Option<u32>,
    ) {
        let exptime = format!("{}", exptime.unwrap_or(0));
        let flags = format!("{}", flags.unwrap_or(0));
        let length = format!("{}", value.len());

        buf.extend_from_slice(b"set ");
        buf.extend_from_slice(key);
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(flags.as_bytes());
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(exptime.as_bytes());
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(length.as_bytes());
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(value);
        buf.extend_from_slice(b"\r\n");
    }
}

impl Default for Memcache {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Memcache {
    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        // Shortest response is "OK\r\n" at 4bytes
        if buf.len() < 4 {
            return Err(Error::Incomplete);
        }

        // All complete responses end in CRLF
        if &buf[buf.len() - 2..buf.len()] != b"\r\n" {
            return Err(Error::Incomplete);
        }

        // count the number of lines
        let reader = BufReader::new(buf.reader());
        let num_lines = reader.lines().count();

        // get lines iterator
        let reader = BufReader::new(buf.reader());
        let mut lines = reader.lines();

        // single line responses
        if num_lines == 1 {
            let line = lines.next().unwrap().unwrap();
            let tokens: Vec<&str> = line.split_whitespace().collect();

            // Single token responses
            if tokens.len() == 1 {
                match &*tokens[0] {
                    "OK" | "STORED" | "DELETED" => {
                        return Ok(Response::Ok);
                    }
                    "END" | "EXISTS" | "NOT_FOUND" | "NOT_STORED" => {
                        return Ok(Response::Miss);
                    }
                    "VALUE" => {
                        // a complete response would have more than one token
                        return Err(Error::Incomplete);
                    }
                    "ERROR" => {
                        return Err(Error::Error);
                    }
                    _ => {}
                }
                // incr/decr give a numeric single token response
                if tokens[0].parse::<u64>().is_ok() {
                    return Ok(Response::Ok);
                }
            } else {
                match &*tokens[0] {
                    "VALUE" => {
                        // a complete response would have more than one line
                        return Err(Error::Incomplete);
                    }
                    "VERSION" => {
                        return Ok(Response::Version);
                    }
                    "CLIENT_ERROR" => {
                        return Err(Error::ClientError);
                    }
                    "SERVER_ERROR" => {
                        return Err(Error::ServerError);
                    }
                    _ => {
                        return Err(Error::Unknown);
                    }
                }
            }
        } else {
            let line = lines.next().unwrap().unwrap();
            let tokens: Vec<&str> = line.split_whitespace().collect();

            match &*tokens[0] {
                "VALUE" => {
                    if tokens.len() < 4 {
                        // first line of VALUE response has 4 tokens
                        return Err(Error::Incomplete);
                    }
                    // Field 3 is the byte length of the response
                    let bytes: usize = match tokens[3].parse() {
                        Ok(b) => b,
                        Err(_) => return Err(Error::Unknown),
                    };
                    // Optional CAS field must be a u64
                    if tokens.len() == 5 {
                        match tokens[4].parse::<u64>() {
                            Ok(_) => {}
                            Err(_) => {
                                return Err(Error::Unknown);
                            }
                        }
                    }
                    // Flags field must be a u32
                    match tokens[2].parse::<u32>() {
                        Ok(_) => {}
                        Err(_) => {
                            return Err(Error::Unknown);
                        }
                    }

                    // All complete responses end in "END\r\n"
                    if &buf[buf.len() - 7..buf.len()] != b"\r\nEND\r\n" {
                        return Err(Error::Incomplete);
                    }

                    let non_data_len = line.len() + 2 + 7; // first line w/ CRLF and last line with both CRLF
                    let data_len = buf.len() - non_data_len;

                    if data_len != bytes {
                        return Err(Error::Unknown);
                    } else {
                        return Ok(Response::Hit);
                    }
                }
                _ => {
                    return Err(Error::Unknown);
                }
            }
        }

        Err(Error::Unknown)
    }

    fn encode(&mut self, buf: &mut BytesMut, rng: &mut ThreadRng) {
        let command = self.generate(rng);
        match command.action() {
            Action::Get => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsGet);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.get(buf, key);
            }
            Action::Set => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsSet);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    metrics.distribution(&Stat::ValueSize, value.len() as u64);
                }
                self.set(buf, key, value, command.ttl().map(|ttl| ttl as u32), None);
            }
            action => {
                fatal!("Action: {:?} unsupported for Memcache", action);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Memcache::new();
            let mut buf = BytesMut::with_capacity(1024);
            buf.extend_from_slice(&message);

            let buf = buf.freeze();
            let result = decoder.decode(&buf);
            assert_eq!(result, response);
        }
    }

    #[test]
    fn decode_incomplete() {
        let messages: Vec<&[u8]> = vec![
            b"",
            b"VALUE ",
            b"STOR",
            b"VALUE 0 0 0\r\nSOME DATA GOES HERE",
            b"VALUE 0 0 0\r\nSOME DATA GOES HERE\r\n",
        ];
        decode_messages(messages, Err(Error::Incomplete));
    }

    #[test]
    fn decode_ok() {
        let messages: Vec<&[u8]> = vec![b"OK\r\n", b"STORED\r\n", b"DELETED\r\n"];
        decode_messages(messages, Ok(Response::Ok));
    }

    #[test]
    fn decode_miss() {
        let messages: Vec<&[u8]> = vec![
            b"END\r\n",
            b"EXISTS\r\n",
            b"NOT_FOUND\r\n",
            b"NOT_STORED\r\n",
        ];
        decode_messages(messages, Ok(Response::Miss));
    }

    #[test]
    fn decode_version() {
        let messages: Vec<&[u8]> = vec![b"VERSION 1.0.0\r\n"];
        decode_messages(messages, Ok(Response::Version));
    }

    #[test]
    fn decode_server_error() {
        let messages: Vec<&[u8]> = vec![b"SERVER_ERROR WHOOPS\r\n"];
        decode_messages(messages, Err(Error::ServerError));
    }

    #[test]
    fn decode_client_error() {
        let messages: Vec<&[u8]> = vec![b"CLIENT_ERROR WHOOPS\r\n"];
        decode_messages(messages, Err(Error::ClientError));
    }

    #[test]
    fn decode_error() {
        let messages: Vec<&[u8]> = vec![b"ERROR\r\n"];
        decode_messages(messages, Err(Error::Error));
    }

    #[test]
    fn decode_unknown() {
        let messages: Vec<&[u8]> = vec![
            b"HELLO WORLD\r\n",
            b"VALUE 0 0 0\r\nSOME DATA GOES HERE\r\nEND\r\n",
            b"VALUE 0 J 8\r\nDEADBEEF\r\nEND\r\n",
            b"VALUE 0 0 8 J\r\nDEADBEEF\r\nEND\r\n",
        ];
        decode_messages(messages, Err(Error::Unknown));
    }

    #[test]
    fn decode_hit() {
        let messages: Vec<&[u8]> = vec![
            b"VALUE 0 0 8\r\nDEADBEEF\r\nEND\r\n",
            b"VALUE 0 0 10\r\nDEAD\r\nBEEF\r\nEND\r\n",
            b"VALUE TEST 0 8\r\nDEADBEEF\r\nEND\r\n",
        ];
        decode_messages(messages, Ok(Response::Hit));
    }

    #[test]
    fn encode_get() {
        let mut buf = BytesMut::new();
        let encoder = Memcache::new();
        encoder.get(&mut buf, b"0");
        assert_eq!(&buf[..], b"get 0\r\n");
    }

    #[test]
    fn encode_set() {
        let mut buf = BytesMut::new();
        let encoder = Memcache::new();
        encoder.set(&mut buf, b"0", b"value", None, None);
        assert_eq!(&buf[..], b"set 0 0 0 5\r\nvalue\r\n");
    }
}
