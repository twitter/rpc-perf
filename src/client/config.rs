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

use super::*;
use super::net::InternetProtocol;
use cfgtypes::*;
use common::*;
use std::sync::Arc;
use tic::{Clocksource, Sender};

const MAX_CONNECTIONS: usize = 65536;
const KILOBYTE: usize = 1024;

#[derive(Clone)]
pub struct Config {
    servers: Vec<String>,
    pool_size: usize,
    stats: Option<Sender<Stat>>,
    clocksource: Option<Clocksource>,
    protocol_name: String,
    protocol: Option<Arc<ProtocolParseFactory>>,
    request_timeout: Option<u64>,
    internet_protocol: InternetProtocol,
    connect_timeout: Option<u64>,
    rx_buffer_size: usize,
    tx_buffer_size: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            servers: Vec::new(),
            pool_size: 1,
            stats: None,
            clocksource: None,
            protocol_name: "unknown".to_owned(),
            protocol: None,
            request_timeout: None,
            connect_timeout: None,
            internet_protocol: InternetProtocol::Any,
            rx_buffer_size: 4 * KILOBYTE,
            tx_buffer_size: 4 * KILOBYTE,
        }
    }
}

impl Config {
    /// add an endpoint (host:port)
    pub fn add_server(&mut self, server: String) -> &mut Self {
        self.servers.push(server);
        self.validate()
    }

    /// get vector of endpoints
    pub fn servers(&self) -> Vec<String> {
        self.servers.clone()
    }

    /// get the number of connections maintained to each endpoint
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }

    /// set the number of connections maintained to each endpoint
    pub fn set_pool_size(&mut self, pool_size: usize) -> &mut Self {
        self.pool_size = pool_size;
        self.validate()
    }

    /// give the client a `Clocksource` for timing
    pub fn set_clocksource(&mut self, clocksource: Clocksource) -> &mut Self {
        self.clocksource = Some(clocksource);
        self
    }

    /// clone the `Clocksource`
    pub fn clocksource(&self) -> Option<Clocksource> {
        self.clocksource.clone()
    }

    /// set the protocol name
    pub fn set_protocol_name(&mut self, name: String) -> &mut Self {
        self.protocol_name = name;
        self
    }

    /// give the client a `ProtocolParseFactory` to read the responses
    pub fn set_protocol(&mut self, protocol: Arc<ProtocolParseFactory>) -> &mut Self {
        self.protocol = Some(protocol);
        self
    }

    /// give the client a `ProtocolParseFactory` to read the responses
    pub fn protocol(&self) -> Option<Arc<ProtocolParseFactory>> {
        self.protocol.clone()
    }

    /// get the InternetProtocol to use for Connections
    pub fn internet_protocol(&self) -> InternetProtocol {
        self.internet_protocol
    }

    /// set the InternetProtocol to use for Connections
    pub fn set_internet_protocol(&mut self, protocol: InternetProtocol) -> &mut Self {
        self.internet_protocol = protocol;
        self
    }

    /// sets the timeout for responses
    pub fn set_request_timeout(&mut self, milliseconds: Option<u64>) -> &mut Self {
        self.request_timeout = milliseconds;
        self
    }

    /// the timeout for responses
    pub fn request_timeout(&self) -> Option<u64> {
        self.request_timeout
    }

    /// sets the timeout for connects
    pub fn set_connect_timeout(&mut self, milliseconds: Option<u64>) -> &mut Self {
        self.connect_timeout = milliseconds;
        self
    }

    /// the timeout for connects
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// sets the rx buffer size in bytes
    pub fn set_rx_buffer_size(&mut self, bytes: usize) -> &mut Self {
        self.rx_buffer_size = bytes;
        self
    }

    /// get the rx buffer size in bytes
    pub fn rx_buffer_size(&self) -> usize {
        self.rx_buffer_size
    }

    /// sets the tx buffer size in bytes
    pub fn set_tx_buffer_size(&mut self, bytes: usize) -> &mut Self {
        self.tx_buffer_size = bytes;
        self
    }

    /// get the tx buffer size in bytes
    pub fn tx_buffer_size(&self) -> usize {
        self.tx_buffer_size
    }

    /// turn the `Config` into a `Client`
    pub fn build(mut self) -> Client {
        self.validate();
        Client::configured(self)
    }

    /// give the client a stats sender
    pub fn set_stats(&mut self, stats: Sender<Stat>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    /// return clone of stats sender
    pub fn stats(&self) -> Option<Sender<Stat>> {
        self.stats.clone()
    }

    /// validation after set methods
    fn validate(&mut self) -> &mut Self {
        if (self.servers.len() * self.pool_size) > MAX_CONNECTIONS {
            halt!("Too many total connections");
        }
        self
    }
}
