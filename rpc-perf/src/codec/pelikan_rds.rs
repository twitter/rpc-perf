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
use crate::config::Action;

use bytes::BytesMut;

pub struct PelikanRds {
    codec: codec::PelikanRds,
    common: Common,
}

impl PelikanRds {
    pub fn new() -> Self {
        Self {
            codec: codec::PelikanRds::new(),
            common: Common::new(),
        }
    }
}

impl Codec for PelikanRds {
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
                    recorder.increment("commands/get");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec.get(buf, key);
            }
            Action::Set => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/set");
                    recorder.distribution("keys/size", key.len() as u64);
                    recorder.distribution("values/size", value.len() as u64);
                }
                self.codec.set(buf, key, value, command.ttl());
            }
            Action::SarrayCreate => {
                let key = command.key().unwrap();
                let values = command.values_strings().unwrap();
                let esize = values[0].len();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/create");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                let watermark_low = if let Some(v) = values.get(1) {
                    match v.parse::<usize>() {
                        Ok(v) => Some(v),
                        _ => None,
                    }
                } else {
                    None
                };
                let watermark_high = if let Some(v) = values.get(2) {
                    match v.parse::<usize>() {
                        Ok(v) => Some(v),
                        _ => None,
                    }
                } else {
                    None
                };
                self.codec
                    .sarray_create(buf, key, esize, watermark_low, watermark_high);
            }
            Action::SarrayDelete => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/delete");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec.sarray_delete(buf, key);
            }
            Action::SarrayFind => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/find");
                    recorder.distribution("keys/size", key.len() as u64);
                    recorder.distribution("values/size", value.len() as u64);
                }
                self.codec.sarray_find(buf, key, value);
            }
            Action::SarrayGet => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/get");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                // TODO: implement index and count
                self.codec.sarray_get(buf, key, None, None);
            }
            Action::SarrayInsert => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/set");
                    recorder.distribution("keys/size", key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution("values/size", len as u64);
                }
                self.codec.sarray_insert(buf, key, &values);
            }
            Action::SarrayLen => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/len");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec.sarray_len(buf, key);
            }
            Action::SarrayRemove => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/remove");
                    recorder.distribution("keys/size", key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution("values/size", len as u64);
                }
                self.codec.sarray_remove(buf, key, &values);
            }
            Action::SarrayTruncate => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment("commands/truncate");
                    recorder.distribution("keys/size", key.len() as u64);
                }
                self.codec
                    .sarray_truncate(buf, key, command.count.unwrap_or(0))
            }
        }
    }
}
