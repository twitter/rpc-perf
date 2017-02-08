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

extern crate rpcperf_cfgtypes as cfgtypes;
extern crate rpcperf_common as common;
extern crate rpcperf_echo as echo;
extern crate rpcperf_redis as redis;
extern crate rpcperf_memcache as memcache;
extern crate rpcperf_ping as ping;
extern crate rpcperf_thrift as thrift;

pub mod config;
pub mod workload;

use common::stats::{Stat, Sender};

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
    pub protocol_config: ProtocolConfig,
    stats: Option<Sender<Stat>>,
    connect_timeout: Option<u64>,
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
            protocol_config: protocol,
            stats: None,
            connect_timeout: None,
        }
    }

    pub fn set_poolsize(&mut self, connections: usize) -> &Self {
        self.connections = connections;
        self
    }

    pub fn set_threads(&mut self, threads: usize) -> &Self {
        self.threads = threads;
        self
    }

    pub fn set_duration(&mut self, seconds: usize) -> &Self {
        self.duration = seconds;
        self
    }

    pub fn set_windows(&mut self, count: usize) -> &Self {
        self.windows = count;
        self
    }

    pub fn set_tcp_nodelay(&mut self, enabled: bool) -> &Self {
        self.tcp_nodelay = enabled;
        self
    }

    pub fn set_stats(&mut self, sender: Sender<Stat>) -> &Self {
        self.stats = Some(sender);
        self
    }

    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }
}
