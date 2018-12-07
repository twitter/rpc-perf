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

use rand::rngs::ThreadRng;
use crate::client::common::Common;
use crate::client::Client;
use crate::codec::Codec;
use crate::session::PlainSession;
use crate::session::Session;

use logger::*;
use mio::Token;
use slab::Slab;

use std::net::SocketAddr;

/// A structure which represents a `Client` which uses plain `Session`s
pub struct PlainClient {
    common: Common,
    sessions: Slab<PlainSession>,
}

impl PlainClient {
    /// Create a new `PlainClient` which will send requests from the queue and parse the responses
    pub fn new(id: usize, codec: Box<Codec>) -> PlainClient {
        Self {
            common: Common::new(id, codec),
            sessions: Slab::new(),
        }
    }
}

impl Client for PlainClient {
    fn add_endpoint(&mut self, endpoint: &SocketAddr) {
        debug!("adding endpoint: {}", endpoint);
        for _ in 0..self.poolsize() {
            let mut session = PlainSession::new(endpoint);
            session.set_nodelay(self.tcp_nodelay());
            let token = self.sessions.insert(session);
            self.connect_enqueue(mio::Token(token));
        }
        self.connect_shuffle();
    }

    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    fn session(&self, token: Token) -> &Session {
        &self.sessions[token.into()]
    }

    fn session_mut(&mut self, token: Token) -> &mut Session {
        &mut self.sessions[token.into()]
    }

    fn does_negotiate(&self) -> bool {
        false
    }

    fn prepare_request(&mut self, token: Token, rng: &mut ThreadRng) {
        self.common.encode(self.sessions[token.into()].write_buf(), rng)
    }
}
