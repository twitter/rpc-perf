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

pub mod config;
pub mod workload;

use cfgtypes::ProtocolConfig;

pub struct BenchmarkConfig {
    connections: usize,
    threads: usize,
    duration: usize,
    windows: usize,
    tcp_nodelay: bool,
    ipv4: bool,
    ipv6: bool,
    connect_timeout: Option<u64>,
    request_timeout: Option<u64>,
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
            connect_timeout: None,
            request_timeout: None,
            protocol_config: protocol,
        }
    }

    pub fn poolsize(&self) -> usize {
        self.connections
    }

    pub fn set_poolsize(&mut self, connections: usize) -> &Self {
        self.connections = connections;
        self
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn set_threads(&mut self, threads: usize) -> &Self {
        self.threads = threads;
        self
    }

    pub fn duration(&self) -> usize {
        self.duration
    }

    pub fn set_duration(&mut self, seconds: usize) -> &Self {
        self.duration = seconds;
        self
    }

    pub fn windows(&self) -> usize {
        self.windows
    }

    pub fn set_windows(&mut self, count: usize) -> &Self {
        self.windows = count;
        self
    }

    pub fn tcp_nodelay(&self) -> bool {
        self.tcp_nodelay
    }

    pub fn set_tcp_nodelay(&mut self, enabled: bool) -> &Self {
        self.tcp_nodelay = enabled;
        self
    }

    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    pub fn set_connect_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.connect_timeout = milliseconds;
        self
    }

    pub fn request_timeout(&self) -> Option<u64> {
        self.request_timeout
    }

    pub fn set_request_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.request_timeout = milliseconds;
        self
    }
}
