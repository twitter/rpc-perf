// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::*;
use crate::config_file::{Protocol, Verb};
use crate::*;

use rand::rngs::SmallRng;
use rand::SeedableRng;

use std::cmp::Ordering;
use std::io::{BufRead, BufReader, Write};
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

    fn command(buf: &mut Session, mode: &Mode, command: &str, args: Vec<Vec<u8>>) {
        match mode {
            Mode::Inline => {
                let _ = buf.write_all(command.to_string().as_bytes());
                for arg in args {
                    let _ = buf.write_all(b" ");
                    let _ = buf.write_all(&arg);
                }
                let _ = buf.write_all(b"\r\n");
            }
            Mode::Resp => {
                let _ = buf.write_all(
                    format!("*{}\r\n${}\r\n{}", 1 + args.len(), command.len(), command).as_bytes(),
                );
                for arg in args {
                    let _ = buf.write_all(format!("\r\n${}\r\n", arg.len()).as_bytes());
                    let _ = buf.write_all(&arg);
                }
                let _ = buf.write_all(b"\r\n");
            }
        }
    }

    fn get(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let args = vec![keyspace.generate_key(rng)];
        Redis::command(buf, mode, "get", args);
    }

    fn mget(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let mut args = Vec::new();
        for _ in 0..keyspace.batch_size() {
            args.push(keyspace.generate_key(rng));
        }
        Redis::command(buf, mode, "mget", args);
    }

    fn set(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let command = "set";
        let mut args = vec![
            keyspace.generate_key(rng),
            keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()),
        ];
        let ttl = keyspace.ttl();
        if ttl != 0 {
            args.push(b"EX".to_vec());
            args.push(format!("{}", ttl).as_bytes().to_vec());
        }
        Redis::command(buf, mode, command, args);
    }

    fn del(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let args = vec![keyspace.generate_key(rng)];
        Redis::command(buf, mode, "del", args);
    }

    fn hget(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let command = "hget";
        let args = vec![
            keyspace.generate_key(rng),
            keyspace
                .generate_inner_key(rng)
                .unwrap_or_else(|| b"".to_vec()),
        ];
        Redis::command(buf, mode, command, args);
    }

    fn hset(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let command = "hset";
        let args = vec![
            keyspace.generate_key(rng),
            keyspace
                .generate_inner_key(rng)
                .unwrap_or_else(|| b"".to_vec()),
            keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()),
        ];
        Redis::command(buf, mode, command, args);
    }

    fn hsetnx(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let command = "hsetnx";
        let args = vec![
            keyspace.generate_key(rng),
            keyspace
                .generate_inner_key(rng)
                .unwrap_or_else(|| b"".to_vec()),
            keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()),
        ];
        Redis::command(buf, mode, command, args);
    }

    fn hdel(rng: &mut SmallRng, mode: &Mode, keyspace: &Keyspace, buf: &mut Session) {
        let command = "hdel";
        let args = vec![
            keyspace.generate_key(rng),
            keyspace
                .generate_inner_key(rng)
                .unwrap_or_else(|| b"".to_vec()),
        ];
        Redis::command(buf, mode, command, args);
    }
}

impl Codec for Redis {
    fn encode(&mut self, buf: &mut Session) {
        let keyspace = self.config.choose_keyspace(&mut self.rng);
        let command = keyspace.choose_command(&mut self.rng);
        match command.verb() {
            Verb::Get => {
                metrics::REQUEST_GET.increment();
                if keyspace.batch_size() == 1 {
                    Self::get(&mut self.rng, &self.mode, keyspace, buf)
                } else {
                    Self::mget(&mut self.rng, &self.mode, keyspace, buf)
                }
            }
            Verb::Set => Self::set(&mut self.rng, &self.mode, keyspace, buf),
            Verb::Delete => Self::del(&mut self.rng, &self.mode, keyspace, buf),
            Verb::Hget => {
                metrics::REQUEST_GET.increment();
                Self::hget(&mut self.rng, &self.mode, keyspace, buf)
            }
            Verb::Hset => Self::hset(&mut self.rng, &self.mode, keyspace, buf),
            Verb::Hsetnx => Self::hsetnx(&mut self.rng, &self.mode, keyspace, buf),
            Verb::Hdel => Self::hdel(&mut self.rng, &self.mode, keyspace, buf),
            _ => {
                unimplemented!()
            }
        }
    }

    fn decode(&self, buffer: &mut Session) -> Result<(), ParseError> {
        // no-copy borrow as a slice
        let buf: &[u8] = (*buffer).buffer();

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
                        Ok("OK") | Ok("PONG") => {
                            let response_end = buf.len();
                            let _ = buffer.consume(response_end);
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
                        Ok(_) => {
                            let response_end = buf.len();
                            let _ = buffer.consume(response_end);
                            Ok(())
                        },
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
                        let _ = buffer.consume(response_end);
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
                                        let _ = buffer.consume(response_end);
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
                        let _ = buffer.consume(response_end);
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
