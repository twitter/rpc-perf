// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub use codec::RedisMode;

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;

use bytes::BytesMut;
use logger::*;

pub struct Redis {
    codec: codec::Redis,
    common: Common,
}

impl Redis {
    pub fn new(mode: RedisMode) -> Self {
        Self {
            codec: codec::Redis::new(mode),
            common: Common::new(),
        }
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
        self.codec.decode(buf)
    }

    fn encode(&mut self, buf: &mut BytesMut, rng: &mut ThreadRng) {
        let command = self.generate(rng);
        match command.action() {
            Action::Delete => {
                let key = command.key().unwrap();
                let keys = vec![key];
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsDelete);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.delete(buf, &keys);
            }
            Action::Get => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsGet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.get(buf, key);
            }
            Action::Llen => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsLen);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.llen(buf, key);
            }
            Action::Lpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsPush);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.lpush(buf, key, &values);
            }
            Action::Lpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsPush);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.lpushx(buf, key, &values);
            }
            Action::Lrange => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsRange);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.codec
                    .lrange(buf, key, 0, command.count.unwrap_or(1) as isize);
            }
            Action::Ltrim => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsTrim);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: proper handling of start and stop
                self.codec
                    .ltrim(buf, key, 0, command.count.unwrap_or(1) as isize);
            }
            Action::Rpush => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsPush);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.rpush(buf, key, &values);
            }
            Action::Rpushx => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsPush);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.rpushx(buf, key, &values);
            }
            Action::Set => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsSet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    recorder.distribution(&Stat::ValueSize, value.len() as u64);
                }
                self.codec.set(buf, key, value, command.ttl());
            }
            action => {
                fatal!("Action: {:?} unsupported for Redis", action);
            }
        }
    }
}
