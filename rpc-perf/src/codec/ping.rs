// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;

#[derive(Default)]
pub struct Ping {
    common: Common,
    codec: codec::Ping,
}

impl Ping {
    pub fn new() -> Ping {
        Self {
            common: Common::new(),
            codec: Default::default(),
        }
    }
}

impl Codec for Ping {
    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        self.codec.decode(buf)
    }

    fn encode(&mut self, buf: &mut BytesMut, _rng: &mut ThreadRng) {
        self.codec.ping(buf);
    }
}
