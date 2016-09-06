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

extern crate mio;
extern crate mpmc;
extern crate slab;
extern crate tic;

use std::net::ToSocketAddrs;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use mio::deprecated::{EventLoop, EventLoopBuilder, Handler};
use mio::tcp::TcpStream;
use mpmc::Queue as BoundedQueue;
use tic::{Clocksource, Sample};

use cfgtypes;
use connection::Connection;
use net::InternetProtocol;
use net;
use stats::Status;
use state::State;

const MAX_CONNECTIONS: usize = 1024;

type Slab<T> = slab::Slab<T, mio::Token>;

#[derive(Clone)]
pub struct ClientConfig {
    pub servers: Vec<String>,
    pub connections: usize,
    pub stats: tic::Sender<Status>,
    pub clocksource: Clocksource,
    pub client_protocol: Arc<cfgtypes::ProtocolParseFactory>,
    pub internet_protocol: InternetProtocol,
    pub work_rx: BoundedQueue<Vec<u8>>,
    pub tcp_nodelay: bool,
    pub mio_config: EventLoopBuilder,
    pub timeout: Option<u64>,
}

pub struct Client {
    connections: Slab<Connection>,
    config: ClientConfig,
    work_rx: BoundedQueue<Vec<u8>>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Client {
        let connections = Slab::with_capacity(MAX_CONNECTIONS);

        Client {
            config: config.clone(),
            connections: connections,
            work_rx: config.work_rx.clone(),
        }
    }

    pub fn run(&mut self) {
        let mut event_loop = self.config.mio_config.clone().build().unwrap();

        let mut failures = 0;
        let mut connects = 0;

        for server in &self.config.servers {
            for _ in 0..self.config.connections {
                let stats = self.config.stats.clone();
                let clocksource = self.config.clocksource.clone();
                let client_protocol = self.config.client_protocol.new();
                let tcp_nodelay = self.config.tcp_nodelay;
                if let Ok(stream) = connect(server.clone(), self.config.internet_protocol) {
                    match self.connections.insert(Connection::new(stream,
                                                                  server.clone(),
                                                                  None,
                                                                  stats,
                                                                  clocksource,
                                                                  client_protocol,
                                                                  tcp_nodelay)) {
                        Ok(token) => {
                            self.connections[token].token = Some(token);
                            event_loop.register(&self.connections[token].socket,
                                          token,
                                          mio::Ready::writable(),
                                          mio::PollOpt::edge() | mio::PollOpt::oneshot())
                                .unwrap();
                            connects += 1;
                        }
                        _ => debug!("too many established connections"),
                    }
                } else {
                    failures += 1;
                }
            }
        }
        info!("Connections: {} Failures: {}", connects, failures);
        if failures == self.config.connections {
            error!("All connections have failed");
            process::exit(1);
        } else {
            event_loop.run(self).unwrap();
        }
    }
}

fn connect(server: String, protocol: InternetProtocol) -> Result<TcpStream, &'static str> {
    let address = &server.to_socket_addrs().unwrap().next().unwrap();
    match net::to_mio_tcp_stream(address, protocol) {
        Ok(stream) => Ok(stream),
        Err(e) => {
            debug!("connect error: {}", e);
            Err("error connecting")
        }
    }
}

impl Handler for Client {
    type Timeout = mio::Token; // timeouts not used
    type Message = (); // cross-thread notifications not used

    fn ready(&mut self,
             event_loop: &mut EventLoop<Client>,
             token: mio::Token,
             events: mio::Ready) {
        trace!("socket ready: token={:?} events={:?}", token, events);

        match self.connections[token].state {
            State::Closed => {
                trace!("reconnecting closed connection");
                let t1 = self.config.clocksource.counter();
                let connection = self.connections.remove(token).unwrap();
                let _ = connection.stats
                    .send(Sample::new(connection.t0, t1, Status::Closed));
                let server = connection.server;

                let stats = self.config.stats.clone();
                let clocksource = self.config.clocksource.clone();
                let client_protocol = self.config.client_protocol.new();
                let tcp_nodelay = self.config.tcp_nodelay;
                if let Ok(stream) = connect(server.clone(), self.config.internet_protocol) {
                    match self.connections.insert(Connection::new(stream,
                                                                  server.clone(),
                                                                  None,
                                                                  stats,
                                                                  clocksource,
                                                                  client_protocol,
                                                                  tcp_nodelay)) {
                        Ok(token) => {
                            self.connections[token].token = Some(token);
                            event_loop.register(&self.connections[token].socket,
                                          token,
                                          mio::Ready::writable(),
                                          mio::PollOpt::edge() | mio::PollOpt::oneshot())
                                .unwrap();
                        }
                        _ => debug!("too many established connections"),
                    }
                }
            }
            State::Reading => {
                self.connections[token].ready(event_loop, events, None);
            }
            State::Writing => {
                match self.work_rx.pop() {
                    Some(work) => {
                        trace!("sending: {:?}", work);
                        if let Some(timeout) = self.config.timeout {
                            self.connections[token].timeout =
                                Some(event_loop.timeout(token, Duration::from_millis(timeout))
                                    .unwrap());
                        }
                        self.connections[token].ready(event_loop, events, Some(work));
                    }
                    None => {
                        trace!("work queue depleted: token: {:?}", token);
                        self.connections[token].reregister(event_loop)
                    }
                }
            }
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<Client>, token: mio::Token) {
        if self.connections[token].state == State::Reading {
            trace!("handle timeout: token: {:?}", token);
            let connection = self.connections.remove(token).unwrap();
            let t1 = self.config.clocksource.counter();
            let _ = connection.stats
                .send(Sample::new(connection.t0, t1, Status::Timeout));
            let server = connection.server;

            let stats = self.config.stats.clone();
            let clocksource = self.config.clocksource.clone();
            let client_protocol = self.config.client_protocol.new();
            let tcp_nodelay = self.config.tcp_nodelay;
            if let Ok(stream) = connect(server.clone(), self.config.internet_protocol) {
                match self.connections.insert({
                    Connection::new(stream,
                                    server.clone(),
                                    None,
                                    stats,
                                    clocksource,
                                    client_protocol,
                                    tcp_nodelay)
                }) {
                    Ok(token) => {
                        self.connections[token].token = Some(token);
                        event_loop.register(&self.connections[token].socket,
                                      token,
                                      mio::Ready::writable(),
                                      mio::PollOpt::edge() | mio::PollOpt::oneshot())
                            .unwrap();
                    }
                    _ => debug!("too many established connections"),
                }
            }
        }
    }
}
