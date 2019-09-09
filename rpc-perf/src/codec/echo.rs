// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;

use bytes::BytesMut;

pub struct Echo {
    codec: codec::Echo,
    common: Common,
}

impl Echo {
    pub fn new() -> Self {
        Self {
            codec: Default::default(),
            common: Common::new(),
        }
    }
}

impl Default for Echo {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Echo {
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
        self.codec.echo(buf, command.key().unwrap());
    }
}
