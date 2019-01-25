//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use super::*;

use bytes::{Buf, BytesMut, IntoBuf};

use std::io::{BufRead, BufReader};
use std::str;

pub enum Mode {
    Inline,
    Resp,
}

pub struct Redis {
    mode: Mode,
}

impl Redis {
    pub fn new(mode: Mode) -> Self {
        Self { mode }
    }

    pub fn get(&self, buf: &mut BytesMut, key: &[u8], _ttl: Option<usize>) {
        match self.mode {
            Mode::Inline => {
                buf.extend_from_slice(b"get ");
                buf.extend_from_slice(key);
                buf.extend_from_slice(b"\r\n");
            }
            Mode::Resp => {
                buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$");
                buf.extend_from_slice(format!("{}", key.len()).as_bytes());
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(key);
                buf.extend_from_slice(b"\r\n");
            }
        }
    }

    pub fn set(&self, buf: &mut BytesMut, key: &[u8], value: &[u8]) {
        match self.mode {
            Mode::Inline => {
                buf.extend_from_slice(b"set ");
                buf.extend_from_slice(key);
                buf.extend_from_slice(b" ");
                buf.extend_from_slice(value);
                buf.extend_from_slice(b"\r\n");
            }
            Mode::Resp => {
                buf.extend_from_slice(b"*3\r\n$3\r\nset\r\n$");
                buf.extend_from_slice(format!("{}", key.len()).as_bytes());
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(key);
                buf.extend_from_slice(b"\r\n$");
                buf.extend_from_slice(format!("{}", value.len()).as_bytes());
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(value);
                buf.extend_from_slice(b"\r\n");
            }
        }
    }
}

impl Decoder for Redis {
    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        let end = &buf[buf.len() - 2..buf.len()];

        // All complete responses end in CRLF
        if &end[..] != b"\r\n" {
            return Err(Error::Incomplete);
        }

        let first_char = &buf[0..1];
        match str::from_utf8(&first_char[..]) {
            Ok("+") => {
                // simple string
                if buf.len() < 5 {
                    Err(Error::Incomplete)
                } else {
                    let msg = &buf[1..buf.len() - 2];
                    match str::from_utf8(&msg[..]) {
                        Ok("OK") | Ok("PONG") => Ok(Response::Ok),
                        _ => Err(Error::Unknown),
                    }
                }
            }
            Ok("-") => {
                // error response
                Err(Error::Error)
            }
            Ok(":") => {
                // numeric response
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok(msg) => match msg.parse::<i64>() {
                        Ok(_) => Ok(Response::Ok),
                        Err(_) => Err(Error::Unknown),
                    },
                    Err(_) => Err(Error::Unknown),
                }
            }
            Ok("$") => {
                // bulk string
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok("-1") => Ok(Response::Miss),
                    Ok(_) => {
                        let reader = BufReader::new(buf.into_buf().reader());
                        let mut lines = reader.lines();
                        let mut line = lines.next().unwrap().unwrap();
                        let _ = line.remove(0);
                        match line.parse::<usize>() {
                            Ok(expected) => {
                                // data len = buf.len() - line.len() - 2x CRLF - 1
                                let have = buf.len() - line.len() - 5;
                                if have < expected {
                                    Err(Error::Incomplete)
                                } else if have > expected {
                                    println!("have: {} expected: {}", have, expected);
                                    Err(Error::Error)
                                } else {
                                    Ok(Response::Hit)
                                }
                            }
                            Err(_) => Err(Error::Unknown),
                        }
                    }
                    Err(_) => Err(Error::Unknown),
                }
            }
            Ok("*") => {
                // arrays
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok("-1") => Ok(Response::Miss),
                    Ok(_) => {
                        // TODO: implement array parsing
                        Err(Error::Unknown)
                    }
                    Err(_) => Err(Error::Unknown),
                }
            }
            _ => Err(Error::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Redis::new(Mode::Resp);
            let mut buf = BytesMut::with_capacity(1024);
            buf.put(&message);

            let buf = buf.freeze();
            let result = decoder.decode(&buf);
            assert_eq!(result, response);
        }
    }

    #[test]
    fn decode_incomplete() {
        let messages: Vec<&[u8]> = vec![b"+OK", b"+OK\r", b"$7\r\nHELLO\r\n"];
        decode_messages(messages, Err(Error::Incomplete));
    }

    #[test]
    fn decode_ok() {
        let messages: Vec<&[u8]> = vec![
            b"+OK\r\n",
            b":12345\r\n",
            // b":-12345\r\n",
        ];
        decode_messages(messages, Ok(Response::Ok));
    }

    #[test]
    fn decode_miss() {
        let messages: Vec<&[u8]> = vec![b"$-1\r\n", b"*-1\r\n"];
        decode_messages(messages, Ok(Response::Miss));
    }

    #[test]
    fn decode_error() {
        let messages: Vec<&[u8]> = vec![b"-ERROR\r\n", b"-ERROR with message\r\n"];
        decode_messages(messages, Err(Error::Error));
    }

    #[test]
    fn decode_unknown() {
        let messages: Vec<&[u8]> = vec![b"?OK\r\n", b":OK\r\n"];
        decode_messages(messages, Err(Error::Unknown));
    }

    #[test]
    fn decode_hit() {
        let messages: Vec<&[u8]> = vec![b"$0\r\n\r\n", b"$1\r\n1\r\n", b"$8\r\nDEADBEEF\r\n"];
        decode_messages(messages, Ok(Response::Hit));
    }
}
