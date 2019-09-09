// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::client::common::Common;
use crate::client::Client;
use crate::codec::Codec;
use crate::session::{PlainSession, Session};

use logger::*;
use mio::Token;
use rand::rngs::ThreadRng;
use slab::Slab;

use std::net::SocketAddr;

/// A structure which represents a `Client` which uses plain `Session`s
pub struct PlainClient {
    common: Common,
    sessions: Slab<PlainSession>,
}

impl PlainClient {
    /// Create a new `PlainClient` which will send requests from the queue and parse the responses
    pub fn new(id: usize, codec: Box<dyn Codec>) -> PlainClient {
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

    fn session(&self, token: Token) -> &dyn Session {
        &self.sessions[token.into()]
    }

    fn session_mut(&mut self, token: Token) -> &mut dyn Session {
        &mut self.sessions[token.into()]
    }

    fn does_negotiate(&self) -> bool {
        false
    }

    fn prepare_request(&mut self, token: Token, rng: &mut ThreadRng) {
        self.common
            .encode(self.sessions[token.into()].write_buf(), rng)
    }
}
