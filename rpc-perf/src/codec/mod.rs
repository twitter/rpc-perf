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

use crate::config::Config;
use crate::config::Generator;
use crate::stats::SimpleRecorder;
use bytes::BytesMut;
use rand::rngs::ThreadRng;

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
pub use codec::Decoder;
pub use codec::Error;
pub use codec::Response;

use crate::config::Action;

pub struct Command {
    action: Action,
    key: Option<String>,
    values: Option<Vec<String>>,
    ttl: Option<usize>,
    index: Option<u64>,
    count: Option<u64>,
}

impl Command {
    pub fn get(key: String) -> Command {
        Command {
            action: Action::Get,
            key: Some(key),
            values: None,
            ttl: None,
            index: None,
            count: None,
        }
    }

    pub fn set(key: String, value: String, ttl: Option<usize>) -> Command {
        Command {
            action: Action::Set,
            key: Some(key),
            values: Some(vec![value]),
            ttl,
            index: None,
            count: None,
        }
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

    pub fn sarray_create(key: String, value: String) -> Command {
        Command {
            action: Action::SarrayCreate,
            key: Some(key),
            values: Some(vec![value]),
            index: None,
            count: None,
            ttl: None,
        }
    }

    pub fn sarray_delete(key: String) -> Command {
        Command {
            action: Action::SarrayDelete,
            key: Some(key),
            values: None,
            index: None,
            count: None,
            ttl: None,
        }
    }

    pub fn sarray_find(key: String, value: String) -> Command {
        Command {
            action: Action::SarrayFind,
            key: Some(key),
            values: Some(vec![value]),
            index: None,
            count: None,
            ttl: None,
        }
    }

    pub fn sarray_get(key: String, index: Option<u64>, count: Option<u64>) -> Command {
        Command {
            action: Action::SarrayGet,
            key: Some(key),
            index,
            count,
            ttl: None,
            values: None,
        }
    }

    pub fn sarray_insert(key: String, values: Vec<String>) -> Command {
        Command {
            action: Action::SarrayInsert,
            key: Some(key),
            index: None,
            count: None,
            ttl: None,
            values: Some(values),
        }
    }

    pub fn sarray_len(key: String) -> Command {
        Command {
            action: Action::SarrayLen,
            key: Some(key),
            values: None,
            index: None,
            count: None,
            ttl: None,
        }
    }

    pub fn sarray_remove(key: String, value: String) -> Command {
        Command {
            action: Action::SarrayRemove,
            key: Some(key),
            values: Some(vec![value]),
            index: None,
            count: None,
            ttl: None,
        }
    }

    pub fn sarray_truncate(key: String, items: u64) -> Command {
        Command {
            action: Action::SarrayTruncate,
            key: Some(key),
            values: None,
            index: None,
            count: Some(items),
            ttl: None,
        }
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
