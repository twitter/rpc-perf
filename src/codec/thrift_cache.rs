// Copyright 2022 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use super::thrift;

use crate::codec::*;
use crate::config::*;
use crate::config_file::Verb;
use crate::*;

use std::io::Write;

use rand::rngs::SmallRng;
use rand::SeedableRng;

pub struct ThriftCache {
    config: Arc<Config>,
    rng: SmallRng,
}

impl ThriftCache {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            rng: SmallRng::from_entropy(),
        }
    }

    fn append(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut values = Vec::new();
        for _ in 0..keyspace.batch_size() {
            values.push(keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()));
        }

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("append");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(&value);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn appendx(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut values = Vec::new();
        for _ in 0..keyspace.batch_size() {
            values.push(keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()));
        }

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("appendx");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(&value);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn count(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let timeout = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("count");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn get(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut fields = Vec::new();
        for _ in 0..keyspace.batch_size() {
            fields.push(
                keyspace
                    .generate_inner_key(rng)
                    .unwrap_or_else(|| b"".to_vec()),
            );
        }
        let timeout = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("get");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(&field);
        }

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn put(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut fields = Vec::new();
        for _ in 0..keyspace.batch_size() {
            fields.push(
                keyspace
                    .generate_inner_key(rng)
                    .unwrap_or_else(|| b"".to_vec()),
            );
        }
        let mut values = Vec::new();
        for _ in 0..keyspace.batch_size() {
            values.push(keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()));
        }
        let timeout = None;
        let timestamp = None;
        let ttl = keyspace.ttl();

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("put");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(&field);
        }

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(4);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(&value);
        }

        if let Some(timestamp) = timestamp {
            buffer.write_bytes(&[thrift::I64]);
            buffer.write_i16(5);
            buffer.write_i64(timestamp);
        }

        if ttl > 0 {
            buffer.write_bytes(&[thrift::I64]);
            buffer.write_i16(6);
            buffer.write_i64(ttl as i64);
        }

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn remove(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut fields = Vec::new();
        for _ in 0..keyspace.batch_size() {
            fields.push(
                keyspace
                    .generate_inner_key(rng)
                    .unwrap_or_else(|| b"".to_vec()),
            );
        }
        let timeout = None;
        let timestamp = None;
        let count = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("remove");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(&field);
        }

        if let Some(timestamp) = timestamp {
            buffer.write_bytes(&[thrift::I64]);
            buffer.write_i16(4);
            buffer.write_i64(timestamp);
        }

        if let Some(count) = count {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(5);
            buffer.write_i32(count);
        }

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn range(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let mut fields = Vec::new();
        for _ in 0..keyspace.batch_size() {
            fields.push(
                keyspace
                    .generate_inner_key(rng)
                    .unwrap_or_else(|| b"".to_vec()),
            );
        }
        let mut values = Vec::new();
        for _ in 0..keyspace.batch_size() {
            values.push(keyspace.generate_value(rng).unwrap_or_else(|| b"".to_vec()));
        }
        let start = None;
        let stop = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("range");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        if let Some(start) = start {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(3);
            buffer.write_i32(start);
        }

        if let Some(stop) = stop {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(4);
            buffer.write_i32(stop);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    #[allow(dead_code)]
    fn scan(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let start_field = None;
        let end_field = None;
        let ascending = None;
        let limit = None;
        let timeout = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("scan");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        if let Some(start_field) = start_field {
            buffer.write_bytes(&[thrift::STRING]);
            buffer.write_i16(3);
            buffer.write_bytes(start_field);
        }

        if let Some(end_field) = end_field {
            buffer.write_bytes(&[thrift::STRING]);
            buffer.write_i16(4);
            buffer.write_bytes(end_field);
        }

        if let Some(ascending) = ascending {
            buffer.write_bytes(&[thrift::BOOL]);
            buffer.write_i16(5);
            buffer.write_bool(ascending);
        }

        if let Some(limit) = limit {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(6);
            buffer.write_i32(limit);
        }

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }

    fn trim(rng: &mut SmallRng, keyspace: &Keyspace, buf: &mut Session) {
        let key = keyspace.generate_key(rng);
        let target_size = 1;
        let trim_from_smallest = true;
        let timeout = None;

        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("range");
        buffer.sequence_id(0);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(1);
        buffer.write_bytes(b"0");

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(&key);

        buffer.write_bytes(&[thrift::I32]);
        buffer.write_i16(3);
        buffer.write_i32(target_size);

        buffer.write_bytes(&[thrift::BOOL]);
        buffer.write_i16(4);
        buffer.write_bool(trim_from_smallest);

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        let _ = buf.write(buffer.as_bytes());
    }
}

impl Codec for ThriftCache {
    fn encode(&mut self, buf: &mut Session) {
        let keyspace = self.config.choose_keyspace(&mut self.rng);
        let command = keyspace.choose_command(&mut self.rng);
        match command.verb() {
            Verb::Rpush => Self::append(&mut self.rng, keyspace, buf),
            Verb::Rpushx => Self::appendx(&mut self.rng, keyspace, buf),
            Verb::Count => Self::count(&mut self.rng, keyspace, buf),
            Verb::Hget => Self::get(&mut self.rng, keyspace, buf),
            Verb::Hset => Self::put(&mut self.rng, keyspace, buf),
            Verb::Hdel => Self::remove(&mut self.rng, keyspace, buf),
            Verb::Lrange => Self::range(&mut self.rng, keyspace, buf),
            Verb::Ltrim => Self::trim(&mut self.rng, keyspace, buf),
            _ => {
                unimplemented!()
            }
        }
    }

    fn decode(&self, buffer: &mut Session) -> Result<(), ParseError> {
        // no-copy borrow as a slice
        let buf: &[u8] = (*buffer).buffer();

        let bytes = buf.len() as u32;
        if bytes > 4 {
            let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);

            match length.checked_add(4_u32) {
                Some(b) => {
                    if b == bytes {
                        Ok(())
                    } else {
                        Err(ParseError::Incomplete)
                    }
                }
                None => Err(ParseError::Unknown),
            }
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
