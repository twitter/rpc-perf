// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;

use bytes::BytesMut;
use logger::*;

pub struct Memcache {
    codec: codec::Memcache,
    common: Common,
}

impl Memcache {
    pub fn new() -> Self {
        Self {
            codec: Default::default(),
            common: Common::new(),
        }
    }
}

impl Default for Memcache {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Memcache {
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
            Action::Get => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsGet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.get(buf, key);
            }
            Action::Set => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsSet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    recorder.distribution(&Stat::ValueSize, value.len() as u64);
                }
                self.codec
                    .set(buf, key, value, command.ttl().map(|ttl| ttl as u32), None);
            }
            action => {
                fatal!("Action: {:?} unsupported for Memcache", action);
            }
        }
    }
}
