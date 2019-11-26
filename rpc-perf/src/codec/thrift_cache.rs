// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Action;

use bytes::BytesMut;
use logger::*;

pub struct ThriftCache {
    codec: codec::ThriftCache,
    common: Common,
}

impl ThriftCache {
    pub fn new() -> Self {
        Self {
            codec: Default::default(),
            common: Common::new(),
        }
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
        self.codec.decode(buf)
    }

    // TODO(bmartin): fix stats
    fn encode(&mut self, buf: &mut BytesMut, rng: &mut ThreadRng) {
        let command = self.generate(rng);
        match command.action() {
            Action::Hget => {
                let pkey = command.key().unwrap();
                let fields = command.fields().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/get");
                    // recorder.distribution("keys/size", pkey.len() as u64);
                }
                self.codec.get(buf, 0, b"0", pkey, &fields, None);
            }
            Action::Hset => {
                let key = command.key().unwrap();
                let fields = command.fields().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/set");
                    // recorder.distribution("keys/size", key.len() as u64);
                    // recorder.distribution("values/size", values.len() as u64);
                }
                self.codec.put(
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
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/delete");
                    // recorder.distribution("keys/size", key.len() as u64);
                    // recorder.distribution("values/size", values.len() as u64);
                }
                self.codec
                    .remove(buf, 0, b"0", key, &fields, None, None, None);
            }
            Action::Lrange => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/range");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec
                    .range(buf, 0, b"0", key, Some(0), command.count.map(|x| x as i32));
            }
            Action::Ltrim => {
                let key = command.key().unwrap();
                let count = command.count().unwrap() as i32;
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/trim");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.codec.trim(buf, 0, b"0", key, count, true, None);
            }
            Action::Rpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/push");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec.append(buf, 0, b"0", key, &values);
            }
            Action::Rpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/push");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec.appendx(buf, 0, b"0", key, &values);
            }
            action => {
                fatal!("Action: {:?} unsupported for ThriftCache", action);
            }
        }
    }
}
