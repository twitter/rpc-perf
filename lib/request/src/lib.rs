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

#[macro_use]
extern crate log;
extern crate getopts;
extern crate toml;

extern crate mpmc;
extern crate ratelimit;
extern crate shuteye;

extern crate rpcperf_cfgtypes as cfgtypes;
extern crate rpcperf_echo as echo;
extern crate rpcperf_redis as redis;
extern crate rpcperf_memcache as memcache;
extern crate rpcperf_ping as ping;
extern crate rpcperf_thrift as thrift;

pub mod config;
pub mod workload;

use cfgtypes::ProtocolConfig;

pub struct BenchmarkConfig {
    pub connections: usize,
    pub threads: usize,
    pub duration: usize,
    pub windows: usize,
    pub tcp_nodelay: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub timeout: Option<u64>,
    pub evtick: u64,
    pub protocol_config: ProtocolConfig,
}

impl BenchmarkConfig {
    fn new(protocol: ProtocolConfig) -> BenchmarkConfig {
        BenchmarkConfig {
            connections: 1,
            threads: 1,
            duration: 60,
            windows: 5,
            tcp_nodelay: false,
            ipv4: true,
            ipv6: true,
            timeout: None,
            evtick: 100,
            protocol_config: protocol,
        }
    }
}
