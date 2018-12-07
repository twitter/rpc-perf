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

use crate::*;
use bytes::BytesMut;

use mio::{Poll, PollOpt, Ready, Token};
use std::io::{Error, Read, Write};

mod plain_session;
mod stream;
#[cfg(feature = "tls")]
mod tls_session;

pub use crate::session::plain_session::PlainSession;
pub use crate::session::stream::Stream;
#[cfg(feature = "tls")]
pub use crate::session::tls_session::TLSSession;

#[derive(Copy, Clone, Debug, PartialEq)]
/// All possible states for a `Session`
pub enum State {
    Closed,
    Connecting,
    Established,
    Negotiating,
    Reading,
    Writing,
}

/// Holds common `Session` related information
pub struct Common {
    state: State,
    timestamp: Option<u64>,
}

impl Common {
    /// Create a new `Common` to hold `Session` related information
    pub fn new() -> Self {
        Self {
            state: State::Closed,
            timestamp: None,
        }
    }

    /// Returns the last set timestamp
    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp
    }

    /// Sets the timestamp to some value
    pub fn set_timestamp(&mut self, timestamp: Option<u64>) {
        self.timestamp = timestamp;
    }

    /// Gets the last set `State`
    pub fn state(&self) -> State {
        self.state
    }

    /// Set the `State` to some new `State`
    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }
}

pub trait Session: Read + Write {
    // implementation specific

    /// Return a reference to the `session::Common` struct
    fn common(&self) -> &Common;
    /// Return a mutable reference to the `session::Common` struct
    fn common_mut(&mut self) -> &mut Common;
    /// Return a reference to the `session::Stream` struct
    fn stream(&self) -> &Stream;
    /// Return a mutable reference to the `session::Stream` struct
    fn stream_mut(&mut self) -> &mut Stream;
    /// Handle any reads necessary for session management
    fn session_read(&mut self) -> Result<(), Error>;
    /// Handle flushing any writes necessary for session management
    fn session_flush(&mut self) -> Result<(), Error>;
    /// Used to check if the `Session` has completed negotiation
    fn is_handshaking(&self) -> bool;
    /// Used to clear the contents of the session buffer
    fn clear_buffer(&mut self);
    /// Reset the session state so it can be reconnected
    fn session_reset(&mut self);
    /// Get a refernce to the read buffer
    fn read_buf(&self) -> &[u8];
    fn read_to(&mut self) -> Result<usize, Error>;
    /// Mutably borrow the tx buffer
    fn write_buf(&mut self) -> &mut BytesMut;

    // stream management

    /// Creates the underlying connection for the `Session`
    fn connect(&mut self) -> Result<(), Error> {
        self.session_reset();
        self.stream_mut().connect()
    }

    fn set_nodelay(&mut self, nodelay: bool) {
        let _ = self.stream_mut().set_nodelay(nodelay);
    }

    // state management

    /// Returns the current `State` of the `Session`
    fn state(&self) -> State {
        self.common().state()
    }

    /// Set the `Session` `State`
    fn set_state(&mut self, state: State) {
        self.common_mut().set_state(state)
    }

    // timestamps

    /// Returns the time the Session was last Written to
    fn timestamp(&self) -> Option<u64> {
        self.common().timestamp()
    }

    /// Sets the timestamp to some value
    fn set_timestamp(&mut self, timestamp: Option<u64>) {
        self.common_mut().set_timestamp(timestamp);
    }

    // event loop registration

    /// Register the `Session` with an event loop
    fn register(
        &self,
        token: Token,
        poll: &Poll,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), Error> {
        self.stream().register(poll, token, interest, opts)
    }

    // Reregister the `Session` with an event loop
    fn reregister(
        &self,
        token: Token,
        poll: &Poll,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), Error> {
        self.stream().reregister(poll, token, interest, opts)
    }

    /// Deregister the `Session` from an event loop
    fn deregister(&self, poll: &Poll) -> Result<(), Error> {
        self.stream().deregister(poll)
    }
}
