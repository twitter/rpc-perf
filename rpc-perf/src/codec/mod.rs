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
use crate::stats::Simple;
use bytes::BytesMut;
use rand::rngs::ThreadRng;
use std::collections::HashMap;

mod echo;
mod memcache;
mod ping;
mod redis;

pub use crate::codec::echo::Echo;
pub use crate::codec::memcache::Memcache;
pub use crate::codec::ping::Ping;
pub use crate::codec::redis::{Redis, RedisMode};
pub use codec::Decoder;
pub use codec::Error;
pub use codec::Response;

pub enum Command {
    Get(String),
    Set(String, String),
    Mget(Vec<String>),
    Mset(HashMap<String, String>),
}

impl Command {
    pub fn get(key: String) -> Command {
        Command::Get(key)
    }

    pub fn set(key: String, value: String) -> Command {
        Command::Set(key, value)
    }

    pub fn mget(keys: Vec<String>) -> Command {
        Command::Mget(keys)
    }

    pub fn mset(kvs: HashMap<String, String>) -> Command {
        Command::Mset(kvs)
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
    fn set_recorder(&mut self, recorder: Simple) {
        self.common_mut().set_recorder(recorder);
    }
}

pub struct Common {
    generator: Generator,
    recorder: Option<Simple>,
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

    pub fn set_recorder(&mut self, recorder: Simple) {
        self.recorder = Some(recorder);
    }

    pub fn recorder(&self) -> &Option<Simple> {
        &self.recorder
    }
}

impl Default for Common {
    fn default() -> Self {
        Self::new()
    }
}
