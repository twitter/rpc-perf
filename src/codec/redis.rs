// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::cmp::Ordering;
use std::io::{BufRead, BufReader};
use std::str;

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;

use bytes::Buf;

pub enum RedisMode {
    Inline,
    Resp,
}

pub struct Redis {
    common: Common,
    mode: RedisMode,
}

impl Redis {
    pub fn new(mode: RedisMode) -> Self {
        Self {
            common: Common::new(),
            mode,
        }
    }

    fn command(&self, buf: &mut Buffer, command: &str, args: &[&[u8]]) {
        match self.mode {
            RedisMode::Inline => {
                buf.put_slice(command.to_string().as_bytes());
                for arg in args {
                    buf.put_slice(b" ");
                    buf.put_slice(arg);
                }
                buf.put_slice(b"\r\n");
            }
            RedisMode::Resp => {
                buf.put_slice(
                    format!("*{}\r\n${}\r\n{}", 1 + args.len(), command.len(), command).as_bytes(),
                );
                for arg in args {
                    buf.put_slice(format!("\r\n${}\r\n", arg.len()).as_bytes());
                    buf.put_slice(arg);
                }
                buf.put_slice(b"\r\n");
            }
        }
    }

    pub fn delete(&self, buf: &mut Buffer, keys: &[&[u8]]) {
        self.command(buf, "delete", keys);
    }

    pub fn get(&self, buf: &mut Buffer, key: &[u8]) {
        let args = vec![key];
        self.command(buf, "get", &args);
    }

    pub fn hget(&self, buf: &mut Buffer, key: &[u8], field: &[u8]) {
        let args = vec![key, field];
        self.command(buf, "hget", &args);
    }

    pub fn hset(&self, buf: &mut Buffer, key: &[u8], field: &[u8], value: &[u8]) {
        let args = vec![key, field, value];
        self.command(buf, "hset", &args);
    }

    pub fn hsetnx(&self, buf: &mut Buffer, key: &[u8], field: &[u8], value: &[u8]) {
        let args = vec![key, field, value];
        self.command(buf, "hsetnx", &args);
    }

    pub fn mget(&self, buf: &mut Buffer, keys: &[&[u8]]) {
        self.command(buf, "mget", keys);
    }

    pub fn set(&self, buf: &mut Buffer, key: &[u8], value: &[u8], ttl: Option<usize>) {
        let mut args = vec![key, value];
        if let Some(ttl) = ttl {
            args.push(b"EX");
            let ttl = format!("{}", ttl);
            args.push(ttl.as_bytes());
            self.command(buf, "set", &args);
        } else {
            self.command(buf, "set", &args);
        }
    }

    pub fn lindex(&self, buf: &mut Buffer, key: &[u8], index: isize) {
        let index = format!("{}", index);
        let args = vec![key, index.as_bytes()];
        self.command(buf, "lindex", &args);
    }

    pub fn llen(&self, buf: &mut Buffer, key: &[u8]) {
        let args = vec![key];
        self.command(buf, "llen", &args);
    }

    pub fn lpop(&self, buf: &mut Buffer, key: &[u8]) {
        let args = vec![key];
        self.command(buf, "lpop", &args);
    }

    pub fn lpush(&self, buf: &mut Buffer, key: &[u8], values: &[&[u8]]) {
        let mut args = vec![key];
        args.extend_from_slice(&values);
        self.command(buf, "lpush", &args);
    }

    pub fn lpushx(&self, buf: &mut Buffer, key: &[u8], values: &[&[u8]]) {
        let mut args = vec![key];
        args.extend_from_slice(&values);
        self.command(buf, "lpushx", &args);
    }

    pub fn lrange(&self, buf: &mut Buffer, key: &[u8], start: isize, stop: isize) {
        let start = format!("{}", start);
        let stop = format!("{}", stop);
        let args = vec![key, start.as_bytes(), stop.as_bytes()];
        self.command(buf, "lrange", &args);
    }

    pub fn lset(&self, buf: &mut Buffer, key: &[u8], index: isize, value: &[u8]) {
        let index = format!("{}", index);
        let args = vec![key, index.as_bytes(), value];
        self.command(buf, "lset", &args);
    }

    pub fn ltrim(&self, buf: &mut Buffer, key: &[u8], start: isize, stop: isize) {
        let start = format!("{}", start);
        let stop = format!("{}", stop);
        let args = vec![key, start.as_bytes(), stop.as_bytes()];
        self.command(buf, "ltrim", &args);
    }

    pub fn rpush(&self, buf: &mut Buffer, key: &[u8], values: &[&[u8]]) {
        let mut args = vec![key];
        args.extend_from_slice(&values);
        self.command(buf, "rpush", &args);
    }

    pub fn rpushx(&self, buf: &mut Buffer, key: &[u8], values: &[&[u8]]) {
        let mut args = vec![key];
        args.extend_from_slice(&values);
        self.command(buf, "rpushx", &args);
    }
}

impl Codec for Redis {
    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

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
                        let reader = BufReader::new(buf.reader());
                        let mut lines = reader.lines();
                        let mut line = lines.next().unwrap().unwrap();
                        let _ = line.remove(0);
                        match line.parse::<usize>() {
                            Ok(expected) => {
                                // data len = buf.len() - line.len() - 2x CRLF - 1
                                let have = buf.len() - line.len() - 5;
                                match have.cmp(&expected) {
                                    Ordering::Less => Err(Error::Incomplete),
                                    Ordering::Equal => Ok(Response::Hit),
                                    Ordering::Greater => Err(Error::Error),
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

    fn encode(&mut self, buf: &mut Buffer, rng: &mut ThreadRng) {
        let command = self.generate(rng);
        match command.action() {
            Action::Delete => {
                let key = command.key().unwrap();
                let keys = vec![key];
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsDelete);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.delete(buf, &keys);
            }
            Action::Get => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsGet);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.get(buf, key);
            }
            Action::Llen => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsLen);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.llen(buf, key);
            }
            Action::Lpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    metrics.distribution(&Stat::ValueSize, len as u64);
                }
                self.lpush(buf, key, &values);
            }
            Action::Lpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    metrics.distribution(&Stat::ValueSize, len as u64);
                }
                self.lpushx(buf, key, &values);
            }
            Action::Lrange => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsRange);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.lrange(buf, key, 0, command.count.unwrap_or(1) as isize);
            }
            Action::Ltrim => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsTrim);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.ltrim(buf, key, 0, command.count.unwrap_or(1) as isize);
            }
            Action::Rpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    metrics.distribution(&Stat::ValueSize, len as u64);
                }
                self.rpush(buf, key, &values);
            }
            Action::Rpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    metrics.distribution(&Stat::ValueSize, len as u64);
                }
                self.rpushx(buf, key, &values);
            }
            Action::Set => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsSet);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                    metrics.distribution(&Stat::ValueSize, value.len() as u64);
                }
                self.set(buf, key, value, command.ttl());
            }
            action => {
                fatal!("Action: {:?} unsupported for Redis", action);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Redis::new(RedisMode::Resp);
            let mut buf = BytesMut::with_capacity(1024);
            buf.extend_from_slice(&message);

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
        let messages: Vec<&[u8]> = vec![b"+OK\r\n", b":12345\r\n"];
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

    #[test]
    fn encode_delete() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"delete abc\r\n");
        let keys: Vec<&[u8]> = vec![b"abc"];
        redis.delete(&mut buf, &keys);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*2\r\n$6\r\ndelete\r\n$3\r\nabc\r\n");
        let keys: Vec<&[u8]> = vec![b"abc"];
        redis.delete(&mut buf, &keys);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_mget() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"mget abc xyz\r\n");
        let keys: Vec<&[u8]> = vec![b"abc", b"xyz"];
        redis.mget(&mut buf, &keys);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$4\r\nmget\r\n$3\r\nabc\r\n$3\r\nxyz\r\n");
        let keys: Vec<&[u8]> = vec![b"abc", b"xyz"];
        redis.mget(&mut buf, &keys);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_ttl_resp() {
        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case
            .put_slice(b"*5\r\n$3\r\nset\r\n$3\r\nabc\r\n$4\r\n1234\r\n$2\r\nEX\r\n$4\r\n9876\r\n");
        redis.set(&mut buf, b"abc", b"1234", Some(9876));

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_resp_without_ttl() {
        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$3\r\nset\r\n$3\r\nabc\r\n$4\r\n1234\r\n");
        redis.set(&mut buf, b"abc", b"1234", None);

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_ttl_inline() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"set xyz 987 EX 1000\r\n");
        redis.set(&mut buf, b"xyz", b"987", Some(1000));

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_inline_without_ttl() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"set qrs 567\r\n");
        redis.set(&mut buf, b"qrs", b"567", None);

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lpush() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lpush abc 123\r\n");
        let values: Vec<&[u8]> = vec![b"123"];
        redis.lpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lpush abc 123 456\r\n");
        let values: Vec<&[u8]> = vec![b"123", b"456"];
        redis.lpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$5\r\nlpush\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        redis.lpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$5\r\nlpush\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n");
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        redis.lpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lpushx() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lpushx abc 123\r\n");
        let values: Vec<&[u8]> = vec![b"123"];
        redis.lpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lpushx abc 123 456\r\n");
        let values: Vec<&[u8]> = vec![b"123", b"456"];
        redis.lpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$6\r\nlpushx\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        redis.lpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$6\r\nlpushx\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n");
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        redis.lpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_ltrim() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"ltrim abc 0 -2\r\n");
        redis.ltrim(&mut buf, b"abc", 0, -2);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$5\r\nltrim\r\n$3\r\nabc\r\n$1\r\n0\r\n$2\r\n-2\r\n");
        redis.ltrim(&mut buf, b"abc", 0, -2);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lrange() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lrange abc 0 -2\r\n");
        redis.lrange(&mut buf, b"abc", 0, -2);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$6\r\nlrange\r\n$3\r\nabc\r\n$1\r\n0\r\n$2\r\n-2\r\n");
        redis.lrange(&mut buf, b"abc", 0, -2);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lset() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lset bee 0 cafe\r\n");
        redis.lset(&mut buf, b"bee", 0, b"cafe");
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$4\r\nlset\r\n$3\r\nbee\r\n$1\r\n0\r\n$4\r\ncafe\r\n");
        redis.lset(&mut buf, b"bee", 0, b"cafe");
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lindex() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lindex bee 0\r\n");
        redis.lindex(&mut buf, b"bee", 0);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$6\r\nlindex\r\n$3\r\nbee\r\n$1\r\n0\r\n");
        redis.lindex(&mut buf, b"bee", 0);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_llen() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"llen bee\r\n");
        redis.llen(&mut buf, b"bee");
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*2\r\n$4\r\nllen\r\n$3\r\nbee\r\n");
        redis.llen(&mut buf, b"bee");
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_lpop() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"lpop bee\r\n");
        redis.lpop(&mut buf, b"bee");
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*2\r\n$4\r\nlpop\r\n$3\r\nbee\r\n");
        redis.lpop(&mut buf, b"bee");
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_rpush() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"rpush abc 123\r\n");
        let values: Vec<&[u8]> = vec![b"123"];
        redis.rpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"rpush abc 123 456\r\n");
        let values: Vec<&[u8]> = vec![b"123", b"456"];
        redis.rpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$5\r\nrpush\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        redis.rpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$5\r\nrpush\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n");
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        redis.rpush(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_rpushx() {
        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"rpushx abc 123\r\n");
        let values: Vec<&[u8]> = vec![b"123"];
        redis.rpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Inline);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"rpushx abc 123 456\r\n");
        let values: Vec<&[u8]> = vec![b"123", b"456"];
        redis.rpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*3\r\n$6\r\nrpushx\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        redis.rpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let redis = Redis::new(RedisMode::Resp);
        let mut buf = Buffer::new();
        let mut test_case = Buffer::new();
        test_case.put_slice(b"*4\r\n$6\r\nrpushx\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n");
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        redis.rpushx(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }
}
