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

use cfgtypes::*;
use common::*;
use common::stats::Stat;
use connection::*;
use mio;
use mio::{Evented, Events, Poll, PollOpt, Token};
use mio::unix::UnixReady;
use mpmc::Queue;
use net::InternetProtocol;
use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tic::{Clocksource, Sample, Sender};

const MAX_CONNECTIONS: usize = 65536;
const MAX_EVENTS: usize = 1024;
const MAX_PENDING: usize = 1024;
const TICK_MS: u64 = 1;

fn pollopt_conn() -> PollOpt {
    PollOpt::edge() | PollOpt::oneshot()
}

type Slab<T> = slab::Slab<T, Token>;

#[derive(Clone)]
pub struct Config {
    servers: Vec<String>,
    pool_size: usize,
    stats: Option<Sender<Stat>>,
    clocksource: Option<Clocksource>,
    protocol: Option<Arc<ProtocolParseFactory>>,
    request_timeout: Option<u64>,
    internet_protocol: InternetProtocol,
    connect_timeout: Option<u64>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            servers: Vec::new(),
            pool_size: 1,
            stats: None,
            clocksource: None,
            protocol: None,
            request_timeout: None,
            connect_timeout: None,
            internet_protocol: InternetProtocol::Any,
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

    /// give the client a `ProtocolParseFactory` to read the responses
    pub fn set_protocol(&mut self, protocol: Arc<ProtocolParseFactory>) -> &mut Self {
        self.protocol = Some(protocol);
        self
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

    /// sets the timeout for connects
    pub fn set_connect_timeout(&mut self, milliseconds: Option<u64>) -> &mut Self {
        self.connect_timeout = milliseconds;
        self
    }

    /// turn the `Config` into a `Client`
    pub fn build(mut self) -> Client {
        self.validate();
        Client::configured(self)
    }

    /// sgive the client a stats sender
    pub fn stats(&mut self, stats: Sender<Stat>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    /// validation after set methods
    fn validate(&mut self) -> &mut Self {
        if (self.servers.len() * self.pool_size) > MAX_CONNECTIONS {
            halt!("Too many total connections");
        }
        self
    }
}

pub struct Client {
    config: Config,
    connections: Slab<Connection>,
    poll: Poll,
    queue: Queue<Vec<u8>>,
    ready: VecDeque<Token>,
    stats: Sender<Stat>,
    times: Vec<u64>,
    rtimes: Vec<u64>,
    clocksource: Clocksource,
    protocol: Box<ProtocolParse>,
    request_timeout: Option<u64>,
    connect_timeout: Option<u64>,
    events: Option<Events>,
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
            halt!("need stats");
        }
        if config.clocksource.is_none() {
            halt!("need clocksource");
        }
        if config.protocol.is_none() {
            halt!("need protocol");
        }

        let c = config.clone();

        let queue = Queue::with_capacity(MAX_PENDING);

        let clocksource = config.clocksource.unwrap();

        let mut client = Client {
            config: c,
            connections: Slab::with_capacity(MAX_CONNECTIONS),
            poll: Poll::new().unwrap(),
            queue: queue,
            ready: VecDeque::new(),
            stats: config.stats.unwrap(),
            times: vec![clocksource.counter(); MAX_CONNECTIONS],
            rtimes: vec![clocksource.counter(); MAX_CONNECTIONS],
            clocksource: clocksource,
            protocol: config.protocol.unwrap().clone().new(),
            request_timeout: config.request_timeout,
            connect_timeout: config.connect_timeout,
            events: Some(Events::with_capacity(MAX_EVENTS)),
        };

        for server in client.config.servers() {
            if let Ok(sock_addr) = client.resolve(server.clone()) {
                for _ in 0..client.config.pool_size() {

                    match client.connections.insert(Connection::new(sock_addr)) {
                        Ok(token) => {
                            client.send_stat(token, Stat::SocketCreate);
                            if client.has_stream(token) {
                                client.register(client.connections[token].stream().unwrap(), token);
                                client.set_timeout(token);
                            } else {
                                error!("failure creating connection");
                            }
                        }
                        Err(_) => {
                            halt!("error acquiring token for connection");
                        }
                    }
                }
            } else {
                panic!("Error resolving: {}", server);
            }
        }
        client
    }

    #[inline]
    fn has_stream(&self, token: Token) -> bool {
        self.connections[token].stream().is_some()
    }

    #[inline]
    fn is_connection(&self, token: Token) -> bool {
        token.0 <= MAX_CONNECTIONS
    }

    fn set_timeout(&mut self, token: Token) {
        if self.is_connection(token) {
            if self.connections[token].is_connecting() {
                if let Some(t) = self.connect_timeout {
                    let deadline = self.clocksource.counter() +
                                   t * self.clocksource.frequency() as u64 / 1000;
                    self.connections[token].set_timeout(Some(deadline));
                }
            } else if let Some(t) = self.request_timeout {
                let deadline = self.clocksource.counter() +
                               t * self.clocksource.frequency() as u64 / 1000;
                self.connections[token].set_timeout(Some(deadline));
            }
        }
    }

    /// register with the poller
    /// - reregister on failure
    fn register<E: ?Sized>(&self, io: &E, token: Token)
        where E: Evented
    {
        match self.poll
                  .register(io, token, self.event_set(token), self.poll_opt(token)) {
            Ok(_) => {}
            Err(e) => {
                if !self.poll.deregister(io).is_ok() {
                    debug!("error registering {:?}: {}", token, e);
                } else {
                    let _ = self.poll
                        .register(io, token, self.event_set(token), self.poll_opt(token));
                }
            }
        }
    }

    // remove from the poller
    fn deregister<E: ?Sized>(&self, io: &E)
        where E: Evented
    {
        match self.poll.deregister(io) {
            Ok(_) => {}
            Err(e) => {
                debug!("error deregistering: {}", e);
            }
        }
    }

    #[inline]
    fn event_set(&self, token: Token) -> mio::Ready {
        self.connections[token].event_set()
    }

    #[inline]
    fn poll_opt(&self, token: Token) -> mio::PollOpt {
        if token.0 <= MAX_CONNECTIONS {
            pollopt_conn()
        } else {
            halt!("poll_opt() unknown token: {:?}", token);
        }
    }

    #[inline]
    fn state(&self, token: Token) -> &State {
        self.connections[token].state()
    }

    #[inline]
    fn set_writable(&mut self, token: Token) {
        self.connections[token].set_writable();
        self.ready.push_back(token);
    }

    fn close(&mut self, token: Token) {
        if let Some(s) = self.connections[token].stream() {
            self.deregister(s);
        }
        self.clear_timer(token);
        let _ = self.connections[token].close();
        self.send_stat(token, Stat::SocketClose);

    }

    /// resolve host:ip to SocketAddr
    fn resolve(&mut self, server: String) -> Result<SocketAddr, &'static str> {
        if let Ok(result) = server.to_socket_addrs() {
            for addr in result {
                match addr {
                    SocketAddr::V4(_) => {
                        if self.config.internet_protocol() == InternetProtocol::Any ||
                           self.config.internet_protocol() == InternetProtocol::IpV4 {
                            return Ok(addr);
                        }
                    }
                    SocketAddr::V6(_) => {
                        if self.config.internet_protocol() == InternetProtocol::Any ||
                           self.config.internet_protocol() == InternetProtocol::IpV6 {
                            return Ok(addr);
                        }
                    }
                }
            }
        }
        Err("failed to convert to socket address")
    }

    /// reconnect helper
    fn reconnect(&mut self, token: Token) {
        debug!("reconnect {:?}", token);
        self.close(token);
        self.times[token.0] = self.clocksource.counter();
        self.connections[token].connect();
        self.send_stat(token, Stat::SocketCreate);
        if self.connections[token].stream().is_some() {
            self.register(self.connections[token].stream().unwrap(), token);
            self.set_timeout(token);
        } else {
            debug!("failure reconnecting");
            self.send_stat(token, Stat::ConnectError);
            self.set_timeout(token); // set a delay to reconnect
        }
    }

    /// write bytes to connection
    /// - reconnect on failure
    /// - transition to Reading if entire buffer written in one call
    fn write(&mut self, token: Token, work: Vec<u8>) {
        trace!("send to {:?}", token);
        self.send_stat(token, Stat::SocketWrite);
        self.times[token.0] = self.clocksource.counter();
        if self.connections[token].write(work).is_ok() {
            self.set_timeout(token);
            if let Some(s) = self.connections[token].stream() {
                self.register(s, token);
            }
            if self.connections[token].is_readable() {
                self.send_stat(token, Stat::RequestSent);
            }
        } else {
            debug!("couldn't write");
            self.send_stat(token, Stat::ConnectError);
            self.reconnect(token);
        }
    }

    /// read and parse response
    /// - reconnect on failure
    /// - transition to Writing when response is complete
    fn read(&mut self, token: Token) {
        if let Ok(response) = self.connections[token].read() {
            if !response.is_empty() {
                let t0 = self.times[token.0];
                let t1 = self.rtimes[token.0];

                let parsed = self.protocol.parse(&response);

                let status = match parsed {
                    ParsedResponse::Ok => Stat::ResponseOk,
                    ParsedResponse::Hit => {
                        let _ = self.stats.send(Sample::new(t0, t1, Stat::ResponseOk));
                        Stat::ResponseOkHit
                    }
                    ParsedResponse::Miss => {
                        let _ = self.stats.send(Sample::new(t0, t1, Stat::ResponseOk));
                        Stat::ResponseOkMiss
                    }
                    _ => Stat::ResponseError,
                };

                if parsed != ParsedResponse::Incomplete {
                    let _ = self.stats.send(Sample::new(t0, t1, status.clone()));
                    if status == Stat::ResponseError {
                        self.reconnect(token);
                    } else {
                        trace!("switch to writable");
                        self.clear_timer(token);
                        self.set_writable(token);
                    }
                }
            }
        } else {
            debug!("read error. reconnect");
            self.send_stat(token, Stat::ConnectError);
            self.reconnect(token);
        }
    }

    /// timeout handler
    /// - reconnect always
    fn timeout(&mut self, token: Token) {
        debug!("timeout {:?}", token);
        match *self.connections[token].state() {
            State::Connecting => {
                self.send_stat(token, Stat::ConnectTimeout);
                self.reconnect(token);
            }
            State::Closed => {
                self.reconnect(token);
            }
            State::Reading => {
                self.send_stat(token, Stat::ResponseTimeout);
                self.reconnect(token);
            }
            State::Writing => {
                debug!("timeout for State::Writing");
            }
        }
    }

    /// write remaining buffer to underlying stream for token
    /// - reconnect on failure
    /// - transition to Reading when write buffer depleated
    fn flush(&mut self, token: Token) {
        trace!("flush {:?}", token);
        self.times[token.0] = self.clocksource.counter();
        if self.connections[token].flush().is_ok() {
            if let Some(s) = self.connections[token].stream() {
                self.register(s, token);
            }
        } else {
            self.send_stat(token, Stat::ConnectError);
            self.reconnect(token);
        }
    }

    /// write a request from the queue to the given token
    fn send(&mut self, token: Token) {
        if self.connections[token].is_writable() {
            if let Some(work) = self.queue.pop() {
                self.write(token, work);
            } else {
                self.set_writable(token);
            }
        } else {
            halt!("internal state error. dispatch to non-writable {:?}",
                   self.state(token));
        }
    }

    fn send_stat(&mut self, token: Token, stat: Stat) {
        let t0 = self.times[token.0];
        let t1 = self.clocksource.counter();
        let _ = self.stats.send(Sample::new(t0, t1, stat));
    }

    fn clear_timer(&mut self, token: Token) {
        self.connections[token].set_timeout(None);
    }

    /// event handler for connections
    fn connection_ready(&mut self, token: Token, event: mio::Event) {
        if self.connections[token].is_connecting() {
            if UnixReady::from(event.readiness()).is_hup() {
                debug!("hangup on connect {:?}", token);
                self.send_stat(token, Stat::ConnectError);
                self.reconnect(token);
                return;
            } else {
                trace!("connection established {:?}", token);
                self.send_stat(token, Stat::ConnectOk);
                self.clear_timer(token);
                self.set_writable(token);
            }
        } else if UnixReady::from(event.readiness()).is_hup() {
            debug!("hangup event {:?}", token);
            self.send_stat(token, Stat::ConnectError);
            self.reconnect(token);
        } else if event.readiness().is_readable() {
            trace!("reading event {:?}", token);
            self.send_stat(token, Stat::SocketRead);
            self.read(token);
        } else if event.readiness().is_writable() {
            trace!("writing event {:?}", token);
            self.send_stat(token, Stat::SocketFlush);
            self.flush(token);
        }
    }

    /// poll for events and handle them
    pub fn poll(&mut self) {
        let time = self.clocksource.counter();

        for i in 0..self.connections.len() {
            if let Some(timeout) = self.connections[Token(i)].get_timeout() {
                if time >= timeout {
                    self.timeout(Token(i));
                }
            }
        }
        let mut events = self.events
            .take()
            .unwrap_or_else(|| Events::with_capacity(MAX_EVENTS));

        self.poll
            .poll(&mut events, Some(Duration::from_millis(TICK_MS)))
            .unwrap();

        let mut rtokens = Vec::new();

        for event in events.iter() {
            let token = event.token();
            if token.0 <= MAX_CONNECTIONS {
                trace!("connection ready {:?}", token);
                self.rtimes[token.0] = self.clocksource.counter();
                rtokens.push((token, event));
            } else {
                halt!("unknown token: {:?}", token);
            }
        }

        for (token, event) in rtokens {
            self.connection_ready(token, event);
        }

        for _ in 0..self.ready.len() {
            let token = self.ready.pop_front().unwrap();
            self.send(token);
        }

        self.events = Some(events);
    }

    /// spins on the poll() function to continuously poll for events
    pub fn run(&mut self) {
        loop {
            self.poll();
        }
    }

    /// returns a synchronous sender for pushing requests to the connection
    pub fn tx(&self) -> Queue<Vec<u8>> {
        self.queue.clone()
    }
}
