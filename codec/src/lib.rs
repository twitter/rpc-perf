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

mod echo;
mod memcache;
mod pelikan_rds;
mod ping;
mod redis;
mod thrift;

pub use crate::echo::Echo;
pub use crate::memcache::Memcache;
pub use crate::pelikan_rds::PelikanRds;
pub use crate::ping::Ping;
pub use crate::redis::Mode as RedisMode;
pub use crate::redis::Redis;
pub use crate::thrift::Cache as ThriftCache;

#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Ok,
    Version,
    Hit,
    Miss,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Incomplete,
    Error,
    ClientError,
    ServerError,
    Unknown,
    ChecksumMismatch(Vec<u8>, Vec<u8>),
}

pub trait Decoder: Send {
    fn decode(&self, buf: &[u8]) -> Result<Response, Error>;
}
