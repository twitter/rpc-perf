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

extern crate slab;
extern crate mio;
extern crate tic;

use std::collections::VecDeque;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use mio::{Evented, Events, Poll, PollOpt, Ready, Token};
use mio::timer::Timer;
use mio::channel::{Receiver, SyncSender};
use tic::{Clocksource, Sample, Sender};

use cfgtypes::*;
use connection::*;
use stats::Status;

const MAX_CONNECTIONS: usize = 65536;
const MAX_EVENTS: usize = 1024;
const MAX_PENDING: usize = 1024;
const TOKEN_TIMER: Token = Token(MAX_CONNECTIONS + 1);
const TOKEN_WORK: Token = Token(MAX_CONNECTIONS + 2);

type Slab<T> = slab::Slab<T, Token>;

pub struct Config {
    servers: Vec<String>,
    pool_size: usize,
    stats: Option<Sender<Status>>,
    clocksource: Option<Clocksource>,
    protocol: Option<Arc<ProtocolParseFactory>>,
    timeout: Option<u64>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            servers: Vec::new(),
            pool_size: 1,
            stats: None,
            clocksource: None,
            protocol: None,
            timeout: None,
        }
    }
}

impl Config {
    /// add an endpoint (host:port)
    pub fn add_server(&mut self, server: String) -> &mut Self {
        self.servers.push(server);
        self.validate()
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

    /// give the client a `ProtocolParseFactory` to read the responses
    pub fn set_protocol(&mut self, protocol: Arc<ProtocolParseFactory>) -> &mut Self {
        self.protocol = Some(protocol);
        self
    }

    /// give the client a `ProtocolParseFactory` to read the responses
    pub fn set_timeout(&mut self, timeout: Option<u64>) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// turn the `Config` into a `Client`
    pub fn build(mut self) -> Client {
        self.validate();
        Client::configured(self)
    }

    /// sgive the client a stats sender
    pub fn stats(&mut self, stats: Sender<Status>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    /// validation after set methods
    fn validate(&mut self) -> &mut Self {
        if (self.servers.len() * self.pool_size) > MAX_CONNECTIONS {
            error!("Too many total connections");
            exit(1);
        }
        self
    }
}

pub struct Client {
    connections: Slab<Connection>,
    poll: Poll,
    rx: Receiver<Vec<u8>>,
    tx: SyncSender<Vec<u8>>,
    ready: VecDeque<Token>,
    stats: Sender<Status>,
    times: Vec<u64>,
    clocksource: Clocksource,
    protocol: Box<ProtocolParse>,
    timer: Timer<Token>,
    timeout: Option<u64>,
}

impl Default for Client {
    fn default() -> Client {
        Client::configured(Config::default())
    }
}

impl Client {
    /// returns the default `Config` for a `Client`
    pub fn configure() -> Config {
        Default::default()
    }

    /// turn a `Config` into a `Client`
    fn configured(config: Config) -> Client {
        if config.stats.is_none() {
            error!("need stats");
            exit(1);
        }
        if config.clocksource.is_none() {
            error!("need clocksource");
            exit(1);
        }
        if config.protocol.is_none() {
            error!("need protocol");
            exit(1);
        }

        let (tx, rx) = mio::channel::sync_channel(MAX_PENDING);

        let mut client = Client {
            connections: Slab::with_capacity(MAX_CONNECTIONS),
            poll: Poll::new().unwrap(),
            rx: rx,
            tx: tx,
            ready: VecDeque::new(),
            stats: config.stats.unwrap(),
            times: vec![0; MAX_CONNECTIONS],
            clocksource: config.clocksource.unwrap(),
            protocol: config.protocol.unwrap().clone().new(),
            timer: mio::timer::Builder::default().tick_duration(Duration::from_millis(1)).build(),
            timeout: config.timeout,
        };

        for server in config.servers {
            for _ in 0..config.pool_size {
                match client.connections.insert(Connection::new(server.clone())) {
                    Ok(token) => {
                        if let Some(s) = client.connections[token].stream() {
                            client.register(s, token, client.connections[token].event_set());
                        }
                    }
                    Err(_) => {
                        error!("error acquiring token for connection");
                        exit(1);
                    }
                }
            }
        }
        let _ = client.poll.register(&client.timer,
                                     TOKEN_TIMER,
                                     Ready::readable(),
                                     PollOpt::level());
        let _ = client.poll.register(&client.rx, TOKEN_WORK, Ready::readable(), PollOpt::level());

        client
    }

    fn register<E: ?Sized>(&self, io: &E, token: Token, interest: Ready)
        where E: Evented
    {
        match self.poll.register(io, token, interest, PollOpt::edge() | PollOpt::oneshot()) {
            Ok(_) => {}
            Err(e) => {
                if !self.poll.deregister(io).is_ok() {
                    error!("error registering {:?}: {}", token, e);
                } else {
                    let _ = self.poll
                        .register(io, token, interest, PollOpt::edge() | PollOpt::oneshot());
                }
            }
        }
    }

    fn deregister<E: ?Sized>(&self, io: &E)
        where E: Evented
    {
        match self.poll.deregister(io) {
            Ok(_) => {}
            Err(e) => {
                error!("error deregistering: {}", e);
            }
        }
    }

    fn reconnect(&mut self, token: Token) {
        debug!("reconnect {:?}", token);
        if let Some(s) = self.connections[token].stream() {
            self.deregister(s);
        }
        self.connections[token].reconnect();
        if let Some(s) = self.connections[token].stream() {
            self.register(s, token, self.connections[token].event_set())
        } else {
            error!("failure reconnecting");
            exit(1);
        }
    }

    fn write(&mut self, token: Token, work: Vec<u8>) {
        trace!("send to {:?}", token);
        self.times[token.0] = self.clocksource.counter();
        if self.connections[token].write(work).is_ok() {
            if let Some(t) = self.timeout {
                self.connections[token].set_timeout(self.timer
                    .set_timeout(Duration::from_millis(t), token)
                    .unwrap());
            }
            if let Some(s) = self.connections[token].stream() {
                self.register(s, token, self.connections[token].event_set());
            }
        } else {
            debug!("couldn't write");
            self.reconnect(token);
        }
    }

    fn read(&mut self, token: Token) {
        if let Ok(response) = self.connections[token].read() {
            if !response.is_empty() {
                let t0 = self.times[token.0];
                let t1 = self.clocksource.counter();

                let parsed = self.protocol.parse(&response);

                let status = match parsed {
                    ParsedResponse::Hit => Status::Hit,
                    ParsedResponse::Ok => Status::Ok,
                    ParsedResponse::Miss => Status::Miss,
                    _ => Status::Error,
                };

                if parsed != ParsedResponse::Incomplete {
                    trace!("switch to writable");
                    let _ = self.stats.send(Sample::new(t0, t1, status));
                    if let Some(timeout) = self.connections[token].get_timeout() {
                        self.timer.cancel_timeout(&timeout);
                    }
                    self.connections[token].set_writable();
                    self.ready.push_back(token);
                }
            }
        } else {
            debug!("read error. reconnect");
            self.reconnect(token);
        }
    }

    fn timeout(&mut self, token: Token) {
        debug!("timeout {:?}", token);
        let t0 = self.times[token.0];
        let t1 = self.clocksource.counter();
        let _ = self.stats.send(Sample::new(t0, t1, Status::Timeout));
        self.reconnect(token);
    }

    fn flush(&mut self, token: Token) {
        trace!("flush {:?}", token);
        self.times[token.0] = self.clocksource.counter();
        if self.connections[token].flush().is_ok() {
            if let Some(s) = self.connections[token].stream() {
                self.register(s, token, self.connections[token].event_set());
            }
        } else {
            self.reconnect(token);
        }
    }

    /// poll for events and handle them
    pub fn poll(&mut self) {
        let mut events = Events::with_capacity(MAX_EVENTS);
        self.poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();

        for event in events.iter() {
            if event.token() == TOKEN_TIMER {
                if let Some(token) = self.timer.poll() {
                    // we have a timeout to handle
                    trace!("timeout fired for {:?}", token);
                    self.timeout(token);
                }
            } else if event.token() == TOKEN_WORK {
                if !self.ready.is_empty() {
                    // we have work to do and a connection to use
                    let token = self.ready.pop_front().unwrap();
                    if self.connections[token].is_writable() {
                        let work = self.rx.try_recv().unwrap();
                        self.write(token, work);
                    } else {
                        error!("internal state error. dispatch to non-writable {:?}",
                               self.connections[token].state());
                        exit(1);
                    }
                }
            } else {
                // we have a readiness for a connection
                let token = event.token();
                trace!("connection ready {:?}", token);
                if self.connections[token].is_connecting() {
                    trace!("connection established {:?}", token);
                    self.connections[token].set_writable();
                    self.deregister(self.connections[token].stream().unwrap());
                    self.ready.push_back(token);
                } else if event.kind().is_readable() {
                    trace!("reading event {:?}", token);
                    self.read(token);
                } else if event.kind().is_writable() {
                    trace!("writing event {:?}", token);
                    self.flush(token);
                }
            }
        }
    }

    /// spins on the poll() function to continuously poll for events
    pub fn run(&mut self) {
        loop {
            self.poll();
        }
    }

    /// returns a synchronous sender for pushing requests to the connection
    pub fn tx(&self) -> mio::channel::SyncSender<Vec<u8>> {
        self.tx.clone()
    }
}
