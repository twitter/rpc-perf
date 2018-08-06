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
use common::*;

pub struct BenchmarkConfig {
    connections: usize,
    threads: usize,
    duration: usize,
    windows: usize,
    tcp_nodelay: bool,
    ipv4: bool,
    ipv6: bool,
    base_connect_timeout: Option<u64>,
    max_connect_timeout: Option<u64>,
    connect_ratelimit: Option<u64>,
    base_request_timeout: Option<u64>,
    max_request_timeout: Option<u64>,
    protocol_name: String,
    pub protocol_config: ProtocolConfig,
    rx_buffer_size: usize,
    tx_buffer_size: usize,
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
            base_connect_timeout: None,
            connect_ratelimit: None,
            max_connect_timeout: None,
            base_request_timeout: None,
            max_request_timeout: None,
            protocol_name: "unknown".to_owned(),
            protocol_config: protocol,
            tx_buffer_size: 4 * KILOBYTE,
            rx_buffer_size: 4 * KILOBYTE,
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

    pub fn base_connect_timeout(&self) -> Option<u64> {
        self.base_connect_timeout
    }

    pub fn set_base_connect_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.base_connect_timeout = milliseconds;
        self
    }

    pub fn max_connect_timeout(&self) -> Option<u64> {
        self.max_connect_timeout
    }

    pub fn set_max_connect_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.max_connect_timeout = milliseconds;
        self
    }

    pub fn set_connect_ratelimit(&mut self, rate: Option<u64>) -> &Self {
        self.connect_ratelimit = rate;
        self
    }

    pub fn connect_ratelimit(&self) -> Option<u64> {
        self.connect_ratelimit
    }

    pub fn base_request_timeout(&self) -> Option<u64> {
        self.base_request_timeout
    }

    pub fn set_base_request_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.base_request_timeout = milliseconds;
        self
    }

    pub fn max_request_timeout(&self) -> Option<u64> {
        self.max_request_timeout
    }

    pub fn set_max_request_timeout(&mut self, milliseconds: Option<u64>) -> &Self {
        self.max_request_timeout = milliseconds;
        self
    }

    pub fn rx_buffer_size(&self) -> usize {
        self.rx_buffer_size
    }

    pub fn set_rx_buffer_size(&mut self, bytes: usize) -> &Self {
        self.rx_buffer_size = bytes;
        self
    }

    pub fn tx_buffer_size(&self) -> usize {
        self.tx_buffer_size
    }

    pub fn set_tx_buffer_size(&mut self, bytes: usize) -> &Self {
        self.tx_buffer_size = bytes;
        self
    }

    pub fn protocol_name(&self) -> String {
        self.protocol_name.clone()
    }
}
