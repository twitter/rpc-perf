// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub use bytes::BufMut;
pub use bytes::BytesMut;

mod echo;
mod memcache;
mod pelikan_rds;
mod ping;
mod redis;

pub use echo::Echo;
pub use memcache::Memcache;
pub use pelikan_rds::PelikanRds;
pub use ping::Ping;
pub use redis::Redis;

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    Incomplete,
    Error,
    Unknown,
}

pub trait Codec: Send {
    fn decode(&self, buf: &mut BytesMut) -> Result<(), ParseError>;
    fn encode(&mut self, buf: &mut BytesMut);
}
