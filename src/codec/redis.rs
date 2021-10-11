// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::*;
use crate::config_file::{Protocol, Verb};
use crate::*;
use std::io::{BufRead, BufReader};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand_distr::Alphanumeric;

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::str;

pub enum Mode {
    Inline,
    Resp,
}

pub struct Redis {
    config: Arc<Config>,
    mode: Mode,
    rng: SmallRng,
}

impl Redis {
    pub fn new(config: Arc<Config>) -> Self {
        let mode = match config.general().protocol() {
            Protocol::Redis | Protocol::RedisInline => Mode::Inline,
            Protocol::RedisResp => Mode::Resp,
            unknown => {
                fatal!("protocol: {:?} is not a redis protocol", unknown);
            }
        };
        Self {
            config,
            mode,
            rng: SmallRng::from_entropy(),
        }
    }

    fn command(buf: &mut BytesMut, mode: &Mode, command: &str, args: &[&[u8]]) {
        match mode {
            Mode::Inline => {
                buf.extend_from_slice(command.to_string().as_bytes());
                for arg in args {
                    buf.extend_from_slice(b" ");
                    buf.extend_from_slice(arg);
                }
                buf.extend_from_slice(b"\r\n");
            }
            Mode::Resp => {
                buf.extend_from_slice(
                    format!("*{}\r\n${}\r\n{}", 1 + args.len(), command.len(), command).as_bytes(),
                );
                for arg in args {
                    buf.extend_from_slice(format!("\r\n${}\r\n", arg.len()).as_bytes());
                    buf.extend_from_slice(arg);
                }
                buf.extend_from_slice(b"\r\n");
            }
        }
    }

    pub fn get(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut BytesMut) {
        let key = rng
            .sample_iter(&Alphanumeric)
            .take(keyspace.length())
            .collect::<Vec<u8>>();
        Redis::command(buf, mode, "get", &[&key]);
    }

    pub fn set(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut BytesMut) {
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
            Redis::command(
                buf,
                mode,
                "set",
                &[&key, &value, b"EX", format!("{}", ttl).as_bytes()],
            );
        } else {
            Redis::command(buf, mode, "set", &[&key, &value]);
        }
    }
}

impl Codec for Redis {
    fn encode(&mut self, buf: &mut BytesMut) {
        let keyspace = self.config.choose_keyspace(&mut self.rng);
        let command = keyspace.choose_command(&mut self.rng);
        match command.verb() {
            Verb::Get => {
                metrics::REQUEST_GET.increment();
                Redis::get(&mut self.rng, &self.mode, keyspace, buf)
            }
            Verb::Set => Redis::set(&mut self.rng, &self.mode, keyspace, buf),
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
                        Ok("OK") | Ok("PONG") => Ok(()),
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
                    Ok("-1") => Ok(()),
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
                    Ok("-1") => Ok(()),
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
