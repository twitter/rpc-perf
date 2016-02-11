//  rpc-perf - RPC Performance Testing
//  Copyright 2015 Twitter, Inc
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

#![crate_type = "lib"]
#![crate_name = "parser"]

#[macro_use]
extern crate log;
extern crate crc;

#[derive(PartialEq, Debug)]
pub enum ParsedResponse {
    Error(String),
    Hit,
    Incomplete,
    Invalid,
    Miss,
    Ok,
    Unknown,
    Version(String),
}

pub trait Parse {
    fn parse(&self) -> ParsedResponse;
}

pub mod echo;
pub mod memcache;
pub mod ping;
pub mod redis;
