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

mod echo;
mod memcache;
mod pelikan_rds;
mod ping;
mod redis;

pub use crate::codec::echo::Echo;
pub use crate::codec::memcache::Memcache;
pub use crate::codec::pelikan_rds::PelikanRds;
pub use crate::codec::ping::Ping;
pub use crate::codec::redis::{Redis, RedisMode};

pub use codec::{Decoder, Error, Response};

use crate::config::{Action, Config, Generator};
use crate::stats::SimpleRecorder;

use bytes::BytesMut;
use rand::rngs::ThreadRng;

pub struct Command {
    action: Action,
    key: Option<String>,
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
    fn set_recorder(&mut self, recorder: SimpleRecorder) {
        self.common_mut().set_recorder(recorder);
    }
}

pub struct Common {
    generator: Generator,
    recorder: Option<SimpleRecorder>,
}

impl Common {
    pub fn new() -> Self {
        Self {
            generator: Config::default().generator(),
            recorder: None,
        }
    }

    pub fn set_generator(&mut self, generator: Generator) {
        self.generator = generator;
    }

    pub fn set_recorder(&mut self, recorder: SimpleRecorder) {
        self.recorder = Some(recorder);
    }

    pub fn recorder(&self) -> &Option<SimpleRecorder> {
        &self.recorder
    }
}

impl Default for Common {
    fn default() -> Self {
        Self::new()
    }
}
