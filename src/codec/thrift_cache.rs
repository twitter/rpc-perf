// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;
use rustcommon_buffer::Buffer;

pub struct ThriftCache {
    common: Common,
}

impl ThriftCache {
    pub fn new() -> Self {
        Self {
            common: Common::new(),
        }
    }

    pub fn append(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        values: &[&[u8]],
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("append");
        buffer.sequence_id(sequence_id);

        // id 1 is a list of request structs
        // list is fixed to 1 request long
        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        //
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(value);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        buf.extend_from_slice(buffer.as_bytes());
    }

    pub fn appendx(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        values: &[&[u8]],
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("appendx");
        buffer.sequence_id(sequence_id);

        // id 1 is a list of request structs
        // list is fixed to 1 request long
        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        //
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(value);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        buf.extend_from_slice(buffer.as_bytes());
    }

    pub fn count(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("appendx");
        buffer.sequence_id(sequence_id);

        // id 1 is a list of request structs
        // list is fixed to 1 request long
        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        //
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        if let Some(timeout) = timeout {
            buffer.write_bytes(&[thrift::I32]);
            buffer.write_i16(11);
            buffer.write_i32(timeout);
        }

        // stop request struct
        buffer.stop();

        buffer.stop();
        buffer.frame();

        buf.extend_from_slice(buffer.as_bytes());
    }

    pub fn get(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        fields: &[&[u8]],
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("get");
        buffer.sequence_id(sequence_id);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(field);
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

        buf.extend_from_slice(buffer.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn put(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        fields: &[&[u8]],
        values: &[&[u8]],
        timestamp: Option<i64>,
        ttl: Option<i64>,
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("put");
        buffer.sequence_id(sequence_id);

        // id 1 is a list of request structs
        // list is fixed to 1 request long
        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        //
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(field);
        }

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(4);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(values.len() as i32);

        for value in values {
            buffer.write_i32(value.len() as i32);
            buffer.write_bytes(value);
        }

        if let Some(timestamp) = timestamp {
            buffer.write_bytes(&[thrift::I64]);
            buffer.write_i16(5);
            buffer.write_i64(timestamp);
        }

        if let Some(ttl) = ttl {
            buffer.write_bytes(&[thrift::I64]);
            buffer.write_i16(6);
            buffer.write_i64(ttl);
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

        buf.extend_from_slice(buffer.as_bytes());
    }

    pub fn range(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        start: Option<i32>,
        stop: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("range");
        buffer.sequence_id(sequence_id);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

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

        buf.extend_from_slice(buffer.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn remove(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        fields: &[&[u8]],
        timestamp: Option<i64>,
        count: Option<i32>,
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("remove");
        buffer.sequence_id(sequence_id);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(3);
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i32(fields.len() as i32);

        for field in fields {
            buffer.write_i32(field.len() as i32);
            buffer.write_bytes(field);
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

        buffer.stop();

        buffer.stop();
        buffer.frame();

        buf.extend_from_slice(buffer.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn scan(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        start_field: Option<&[u8]>,
        end_field: Option<&[u8]>,
        ascending: Option<bool>,
        limit: Option<i32>,
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("appendx");
        buffer.sequence_id(sequence_id);

        // id 1 is a list of request structs
        // list is fixed to 1 request long
        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        //
        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

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

        buf.extend_from_slice(buffer.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trim(
        &self,
        buf: &mut Buffer,
        sequence_id: i32,
        table: &[u8],
        key: &[u8],
        target_size: i32,
        trim_from_smallest: bool,
        timeout: Option<i32>,
    ) {
        let mut buffer = thrift::ThriftBuffer::new();
        buffer.protocol_header();
        buffer.method_name("trim");
        buffer.sequence_id(sequence_id);

        buffer.write_bytes(&[thrift::LIST]);
        buffer.write_i16(1);
        buffer.write_bytes(&[thrift::STRUCT]);
        buffer.write_i32(1);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(1);
        buffer.write_i32(table.len() as i32);
        buffer.write_bytes(table);

        buffer.write_bytes(&[thrift::STRING]);
        buffer.write_i16(2);
        buffer.write_i32(key.len() as i32);
        buffer.write_bytes(key);

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

        buffer.stop();

        buffer.stop();
        buffer.frame();

        buf.extend_from_slice(buffer.as_bytes());
    }
}

impl Default for ThriftCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for ThriftCache {
    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        let bytes = buf.len() as u32;
        if bytes > 4 {
            let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);

            match length.checked_add(4_u32) {
                Some(b) => {
                    if b == bytes {
                        Ok(Response::Ok)
                    } else {
                        Err(Error::Incomplete)
                    }
                }
                None => Err(Error::Unknown),
            }
        } else {
            Err(Error::Incomplete)
        }
    }

    // TODO(bmartin): fix stats
    fn encode(&mut self, buf: &mut Buffer, rng: &mut ThreadRng) {
        let command = self.generate(rng);
        match command.action() {
            Action::Hget => {
                let pkey = command.key().unwrap();
                let fields = command.fields().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsGet);
                    // metrics.distribution("keys/size", pkey.len() as u64);
                }
                self.get(buf, 0, b"0", pkey, &fields, None);
            }
            Action::Hset => {
                let key = command.key().unwrap();
                let fields = command.fields().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsSet);
                    // metrics.distribution("keys/size", key.len() as u64);
                    // metrics.distribution("values/size", values.len() as u64);
                }
                self.put(
                    buf,
                    0,
                    b"0",
                    key,
                    &fields,
                    &values,
                    None,
                    command.ttl().map(|ttl| ttl as i64),
                    None,
                );
            }
            Action::Hdel => {
                let key = command.key().unwrap();
                let fields = command.fields().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsDelete);
                    // metrics.distribution("keys/size", key.len() as u64);
                    // metrics.distribution("values/size", values.len() as u64);
                }
                self.remove(buf, 0, b"0", key, &fields, None, None, None);
            }
            Action::Lrange => {
                let key = command.key().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsRange);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.range(buf, 0, b"0", key, Some(0), command.count.map(|x| x as i32));
            }
            Action::Ltrim => {
                let key = command.key().unwrap();
                let count = command.count().unwrap() as i32;
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsTrim);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.trim(buf, 0, b"0", key, count, true, None);
            }
            Action::Rpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.append(buf, 0, b"0", key, &values);
            }
            Action::Rpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(metrics) = self.common.metrics() {
                    metrics.increment(&Stat::CommandsPush);
                    metrics.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.appendx(buf, 0, b"0", key, &values);
            }
            action => {
                fatal!("Action: {:?} unsupported for ThriftCache", action);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get() {
        let mut buf = Buffer::new();
        let codec = ThriftCache::new();
        codec.get(&mut buf, 0, b"0", b"key", &[b"alpha"], None);
        let mut check = Buffer::new();

        check.extend_from_slice(&[
            0, 0, 0, 60, // len
            128, 1, 0, 1, // protocol
            0, 0, 0, 3, // method length
            103, 101, 116, // "get"
            0, 0, 0, 0, // sequence id
            // request is a list of structs
            15, // list
            0, 1,  // id 1 (i16)
            12, // of structs
            0, 0, 0, 1, // list length (i32)
            // first field in the struct is the table
            11, // start string
            0, 1, // id 1 (i16)
            0, 0, 0, 1,  // length 1 byte (i32)
            48, // second field in struct is the key
            11, // start string
            0, 2, // id 2 (i16)
            0, 0, 0, 3, // length 3 bytes (i32)
            107, 101, 121, // "key",
            // third field in struct is a list of fields (lkeys)
            15, // list
            0, 3,  // id 3 (i16),
            11, // of strings
            0, 0, 0, 1, // list length (i32)
            // first lkey
            0, 0, 0, 5, // length 5 bytes (i32)
            97, 108, 112, 104, 97, // "alpha"
            // stop get request struct
            0, // stop requests
            0,
        ]);
        assert_eq!(buf, check);
    }
}
