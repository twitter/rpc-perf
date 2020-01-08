// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config::Action;
use crate::stats::Stat;

use bytes::BytesMut;
use logger::*;

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
                self.codec.set(buf, key, value, command.ttl());
            }
            Action::SarrayCreate => {
                let key = command.key().unwrap();
                let esize = command.esize().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsDelete);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.sarray_create(
                    buf,
                    key,
                    esize,
                    command.watermark_low(),
                    command.watermark_high(),
                );
            }
            Action::SarrayDelete => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsDelete);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.sarray_delete(buf, key);
            }
            Action::SarrayFind => {
                let key = command.key().unwrap();
                let value = command.value().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsFind);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    recorder.distribution(&Stat::ValueSize, value.len() as u64);
                }
                self.codec.sarray_find(buf, key, value);
            }
            Action::SarrayGet => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsGet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                // TODO: implement index and count
                self.codec.sarray_get(buf, key, None, None);
            }
            Action::SarrayInsert => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsSet);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.sarray_insert(buf, key, &values);
            }
            Action::SarrayLen => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsLen);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec.sarray_len(buf, key);
            }
            Action::SarrayRemove => {
                let key = command.key().unwrap();
                let values = command.values().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsRemove);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                    let len: usize = values.iter().map(|v| v.len()).sum();
                    recorder.distribution(&Stat::ValueSize, len as u64);
                }
                self.codec.sarray_remove(buf, key, &values);
            }
            Action::SarrayTruncate => {
                let key = command.key().unwrap();
                if let Some(recorder) = self.common.recorder() {
                    recorder.increment(&Stat::CommandsTruncate);
                    recorder.distribution(&Stat::KeySize, key.len() as u64);
                }
                self.codec
                    .sarray_truncate(buf, key, command.count.unwrap_or(0))
            }
            action => {
                fatal!("Action: {:?} unsupported for pelikan_rds", action);
            }
        }
    }
}
