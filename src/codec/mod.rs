// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod echo;
mod memcache;
mod ping;
mod redis;

use crate::Session;
pub use echo::Echo;
pub use memcache::Memcache;
pub use ping::Ping;
pub use redis::Redis;

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    Incomplete,
    Error,
    Unknown,
}

pub trait Codec: Send {
    fn decode(&self, buf: &mut Session) -> Result<(), ParseError>;
    fn encode(&mut self, buf: &mut Session);
}
