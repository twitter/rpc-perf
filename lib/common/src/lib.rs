//  rpc-perf - RPC Performance Testing
//  Copyright 2017 Twitter, Inc
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

extern crate byteorder;
extern crate crc;
extern crate getopts;
extern crate mio;
extern crate mpmc;
extern crate pad;
extern crate ratelimit;
extern crate toml;
extern crate rand;
extern crate tic;

pub mod random {
    pub use rand::*;
}
pub mod checksum {
    pub use crc::*;
}
pub mod padding {
    pub use pad::*;
}
pub mod bytes {
    pub use byteorder::*;
}
pub mod options {
    pub use getopts::*;
}
pub mod limits {
    pub use ratelimit::*;
}
pub mod config {
    pub use toml::*;
}
pub mod async {
    pub use mio::{Event, Evented, Events, Poll, PollOpt, Ready, Token};
    pub use mio::{channel, tcp, timer};
    pub use mpmc::Queue;
}
pub mod stats {
    use std::fmt;
    pub use tic::*;

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub enum Stat {
        ResponseOk,
        ResponseError,
        ResponseTimeout,
        ResponseOkHit,
        ResponseOkMiss,
        ConnectOk,
        ConnectError,
        ConnectTimeout,
        RequestSent,
        RequestPrepared,
        SocketRead,
        SocketWrite,
        SocketFlush,
        SocketClose,
        SocketCreate,
        Window,
    }

    impl fmt::Display for Stat {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Stat::Window => write!(f, "window"),
                Stat::ResponseOk => write!(f, "response_ok"),
                Stat::ResponseError => write!(f, "response_error"),
                Stat::ResponseTimeout => write!(f, "response_timeout"),
                Stat::ResponseOkHit => write!(f, "response_ok_hit"),
                Stat::ResponseOkMiss => write!(f, "response_ok_miss"),
                Stat::ConnectOk => write!(f, "connect_ok"),
                Stat::ConnectError => write!(f, "connect_error"),
                Stat::ConnectTimeout => write!(f, "connect_timeout"),
                Stat::RequestSent => write!(f, "request_sent"),
                Stat::RequestPrepared => write!(f, "request_prepared"),
                Stat::SocketRead => write!(f, "socket_read"),
                Stat::SocketWrite => write!(f, "socket_write"),
                Stat::SocketFlush => write!(f, "socket_flush"),
                Stat::SocketClose => write!(f, "socket_close"),
                Stat::SocketCreate => write!(f, "socket_create"),
            }
        }
    }
}

pub use mpmc::Queue;
