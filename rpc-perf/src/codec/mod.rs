// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod echo;
mod memcache;
mod pelikan_rds;
mod ping;
mod redis;
mod thrift_cache;

pub use crate::codec::echo::Echo;
pub use crate::codec::memcache::Memcache;
pub use crate::codec::pelikan_rds::PelikanRds;
pub use crate::codec::ping::Ping;
pub use crate::codec::redis::{Redis, RedisMode};
pub use crate::codec::thrift_cache::ThriftCache;

pub use codec::{Decoder, Error, Response};

use crate::config::{Action, Config, Generator};
use crate::stats::Metrics;

use bytes::BytesMut;
use rand::rngs::ThreadRng;

pub struct Command {
    action: Action,
    key: Option<String>,
    fields: Option<Vec<String>>,
    values: Option<Vec<String>>,
    ttl: Option<usize>,
    index: Option<u64>,
    count: Option<u64>,
    esize: Option<usize>,
    watermark_low: Option<usize>,
    watermark_high: Option<usize>,
}

impl Command {
    pub fn new(action: Action) -> Command {
        Command {
            action,
            key: None,
            fields: None,
            values: None,
            ttl: None,
            index: None,
            count: None,
            esize: None,
            watermark_low: None,
            watermark_high: None,
        }
    }

    pub fn delete(key: String) -> Command {
        let mut command = Command::new(Action::Delete);
        command.key = Some(key);
        command
    }

    pub fn get(key: String) -> Command {
        let mut command = Command::new(Action::Get);
        command.key = Some(key);
        command
    }

    pub fn hdel(key: String, fields: Vec<String>) -> Command {
        let mut command = Command::new(Action::Hdel);
        command.key = Some(key);
        command.fields = Some(fields);
        command
    }

    pub fn hget(key: String, fields: Vec<String>) -> Command {
        let mut command = Command::new(Action::Hget);
        command.key = Some(key);
        command.fields = Some(fields);
        command
    }

    pub fn hset(
        key: String,
        fields: Vec<String>,
        values: Vec<String>,
        ttl: Option<usize>,
    ) -> Command {
        let mut command = Command::new(Action::Hget);
        command.key = Some(key);
        command.fields = Some(fields);
        command.values = Some(values);
        command.ttl = ttl;
        command
    }

    pub fn fields(&self) -> Option<Vec<&[u8]>> {
        match &self.fields {
            Some(fields) => {
                let mut v = Vec::new();
                for field in fields {
                    v.push(field.as_bytes())
                }
                Some(v)
            }
            None => None,
        }
    }

    pub fn llen(key: String) -> Command {
        let mut command = Command::new(Action::Llen);
        command.key = Some(key);
        command
    }

    pub fn lpush(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::Lpush);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn lpushx(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::Lpushx);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn lrange(key: String, index: usize, count: usize) -> Command {
        let mut command = Command::new(Action::Lrange);
        command.key = Some(key);
        command.count = Some(count as u64);
        command.index = Some(index as u64);
        command
    }

    pub fn ltrim(key: String, index: usize, count: usize) -> Command {
        let mut command = Command::new(Action::Lrange);
        command.key = Some(key);
        command.count = Some(count as u64);
        command.index = Some(index as u64);
        command
    }

    pub fn rpush(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::Rpush);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn rpushx(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::Rpushx);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn set(key: String, value: String, ttl: Option<usize>) -> Command {
        let mut command = Command::new(Action::Set);
        command.key = Some(key);
        command.values = Some(vec![value]);
        command.ttl = ttl;
        command
    }

    pub fn action(&self) -> Action {
        self.action
    }

    pub fn count(&self) -> Option<u64> {
        self.count
    }

    pub fn key(&self) -> Option<&[u8]> {
        match &self.key {
            Some(key) => Some(key.as_bytes()),
            None => None,
        }
    }

    pub fn value(&self) -> Option<&[u8]> {
        match &self.values {
            Some(values) => Some(values[0].as_bytes()),
            None => None,
        }
    }

    pub fn values(&self) -> Option<Vec<&[u8]>> {
        match &self.values {
            Some(values) => {
                let mut v = Vec::new();
                for value in values {
                    v.push(value.as_bytes())
                }
                Some(v)
            }
            None => None,
        }
    }

    pub fn ttl(&self) -> Option<usize> {
        self.ttl
    }

    pub fn sarray_create(
        key: String,
        esize: usize,
        watermark_low: Option<usize>,
        watermark_high: Option<usize>,
    ) -> Command {
        let mut command = Command::new(Action::SarrayCreate);
        command.key = Some(key);
        command.esize = Some(esize);
        command.watermark_low = watermark_low;
        command.watermark_high = watermark_high;
        command
    }

    pub fn sarray_delete(key: String) -> Command {
        let mut command = Command::new(Action::SarrayDelete);
        command.key = Some(key);
        command
    }

    pub fn sarray_find(key: String, value: String) -> Command {
        let mut command = Command::new(Action::SarrayFind);
        command.key = Some(key);
        command.values = Some(vec![value]);
        command
    }

    pub fn sarray_get(key: String, index: Option<u64>, count: Option<u64>) -> Command {
        let mut command = Command::new(Action::SarrayGet);
        command.key = Some(key);
        command.index = index;
        command.count = count;
        command
    }

    pub fn sarray_insert(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::SarrayInsert);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn sarray_len(key: String) -> Command {
        let mut command = Command::new(Action::SarrayLen);
        command.key = Some(key);
        command
    }

    pub fn sarray_remove(key: String, values: Vec<String>) -> Command {
        let mut command = Command::new(Action::SarrayRemove);
        command.key = Some(key);
        command.values = Some(values);
        command
    }

    pub fn sarray_truncate(key: String, items: u64) -> Command {
        let mut command = Command::new(Action::SarrayTruncate);
        command.key = Some(key);
        command.count = Some(items);
        command
    }

    pub fn esize(&self) -> Option<usize> {
        self.esize
    }

    pub fn watermark_low(&self) -> Option<usize> {
        self.watermark_low
    }

    pub fn watermark_high(&self) -> Option<usize> {
        self.watermark_high
    }
}

pub trait Codec: Send {
    fn common(&self) -> &Common;
    fn common_mut(&mut self) -> &mut Common;
    fn decode(&self, buf: &[u8]) -> Result<Response, Error>;
    fn encode(&mut self, buf: &mut BytesMut, rng: &mut ThreadRng);

    fn generate(&self, rng: &mut ThreadRng) -> Command {
        self.common().generator.generate(rng)
    }
    fn set_generator(&mut self, generator: Generator) {
        self.common_mut().set_generator(generator);
    }
    fn set_metrics(&mut self, metrics: Metrics) {
        self.common_mut().set_metrics(metrics);
    }
}

pub struct Common {
    generator: Generator,
    metrics: Option<Metrics>,
}

impl Common {
    pub fn new() -> Self {
        Self {
            generator: Config::default().generator(),
            metrics: None,
        }
    }

    pub fn set_generator(&mut self, generator: Generator) {
        self.generator = generator;
    }

    pub fn set_metrics(&mut self, metrics: Metrics) {
        self.metrics = Some(metrics);
    }

    pub fn recorder(&self) -> &Option<Metrics> {
        &self.metrics
    }
}

impl Default for Common {
    fn default() -> Self {
        Self::new()
    }
}
