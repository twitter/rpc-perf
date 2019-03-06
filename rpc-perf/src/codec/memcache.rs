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

use crate::codec::*;

use bytes::BytesMut;

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
        match command {
            Command::Get(key) => {
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/get");
                    recorder.distribution("keys/size", key.len());
                }
                self.codec.get(buf, key.as_bytes());
            }
            Command::Set(key, value) => {
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/set");
                    recorder.distribution("keys/size", key.len());
                    recorder.distribution("values/size", value.len());
                }
                self.codec.set(buf, key.as_bytes(), value.as_bytes(), None, None);
            }
            _ => unimplemented!("command is not implemented for the memcache codec."),
        }
    }
}
