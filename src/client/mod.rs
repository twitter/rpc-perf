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

pub mod buffer;
pub mod config;
pub mod connection;
pub mod net;

use self::config::Config;
use self::connection::*;
use self::net::InternetProtocol;
use cfgtypes::*;
use common::stats::Stat;
use mio;
use mio::unix::UnixReady;
use mio::{Evented, Events, Poll, PollOpt, Token};
use mpmc::Queue;
use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tic::{Clocksource, Sample, Sender};
use ratelimit;

const MAX_CONNECTIONS: usize = 65_536;
const MAX_EVENTS: usize = 1024;
const MAX_PENDING: usize = 1024;
const TICK_MS: u64 = 1;

fn pollopt_conn() -> PollOpt {
    PollOpt::edge() | PollOpt::oneshot()
}

type Slab<T> = slab::Slab<T, Token>;

pub struct Client {
    config: Config,
    connections: Slab<Connection>,
    factory: Factory,
    poll: Poll,
    queue: Queue<Vec<u8>>,
    ready: VecDeque<Token>,
    stats: Sender<Stat>,
    times: Vec<u64>,
    rtimes: Vec<u64>,
    clocksource: Clocksource,
    protocol: Box<ProtocolParse>,
    connect_timeout: Option<u64>,
    connect_ratelimit: Option<ratelimit::Handle>,
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
        if config.stats().is_none() {
            halt!("need stats");
        }
        if config.clocksource().is_none() {
            halt!("need clocksource");
        }
        if config.protocol().is_none() {
            halt!("need protocol");
        }

        let c = config.clone();

        let queue = Queue::with_capacity(MAX_PENDING);

        let clocksource = config.clocksource().unwrap();

        let factory = Factory::new(
            config.rx_buffer_size(),
            config.tx_buffer_size(),
            config.base_connect_timeout().unwrap_or(0),
            config.base_request_timeout().unwrap_or(0),
            config.max_connect_timeout(),
            config.max_request_timeout(),
            );

        let mut client = Client {
            clocksource: clocksource.clone(),
            config: c,
            connections: Slab::with_capacity(MAX_CONNECTIONS),
            events: Some(Events::with_capacity(MAX_EVENTS)),
            factory: factory,
            poll: Poll::new().unwrap(),
            queue: queue,
            ready: VecDeque::new(),
            stats: config.stats().unwrap(),
            times: vec![clocksource.counter(); MAX_CONNECTIONS],
            rtimes: vec![clocksource.counter(); MAX_CONNECTIONS],
            protocol: Arc::clone(&config.protocol().unwrap()).new(),
            connect_timeout: config.base_connect_timeout(),
            connect_ratelimit: config.connect_ratelimit(),
        };

        for server in client.config.servers() {
            if let Ok(sock_addr) = client.resolve(server.clone()) {
                for _ in 0..client.config.pool_size() {
                    if let Some(mut ratelimit) = client.connect_ratelimit.clone() {
                        ratelimit.wait();
                    }
                    let connection = client.factory.connect(sock_addr);
                    match client.connections.insert(connection) {
                        Ok(token) => {
                            client.send_stat(token, Stat::SocketCreate);
                            if client.has_stream(token) {
                                client.register(client.connections[token].stream().unwrap(), token);
                                client.set_timeout(token);
                            } else {
                                error!("failure creating connection");
                                client.connections[token].connect_failed();
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
               let t = self.connections[token].connect_timeout() as u64;
               debug!("set connect timeout {:?}: {}", token, t);
                let deadline =
                    self.clocksource.counter() + t * self.clocksource.frequency() as u64 / 1000;
                self.connections[token].set_timeout(Some(deadline));
            } else {
                let t = self.connections[token].request_timeout() as u64;
                debug!("set request timeout {:?}: {}", token, t);
                let deadline =
                    self.clocksource.counter() + t * self.clocksource.frequency() as u64 / 1000;
                self.connections[token].set_timeout(Some(deadline));
            }
        }
    }

    /// register with the poller
    /// - reregister on failure
    fn register<E: ?Sized>(&self, io: &E, token: Token)
    where
        E: Evented,
    {
        match self
            .poll
            .register(io, token, self.event_set(token), self.poll_opt(token))
        {
            Ok(_) => {}
            Err(e) => {
                if !self.poll.deregister(io).is_ok() {
                    debug!("error registering {:?}: {}", token, e);
                } else {
                    let _ =
                        self.poll
                            .register(io, token, self.event_set(token), self.poll_opt(token));
                }
            }
        }
    }

    // remove from the poller
    fn deregister<E: ?Sized>(&self, io: &E)
    where
        E: Evented,
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
    fn state(&self, token: Token) -> State {
        self.connections[token].state()
    }

    #[inline]
    fn set_state(&mut self, token: Token, state: State) {
        self.connections[token].set_state(state);
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
                        if self.config.internet_protocol() == InternetProtocol::Any
                            || self.config.internet_protocol() == InternetProtocol::IpV4
                        {
                            return Ok(addr);
                        }
                    }
                    SocketAddr::V6(_) => {
                        if self.config.internet_protocol() == InternetProtocol::Any
                            || self.config.internet_protocol() == InternetProtocol::IpV6
                        {
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
        let wait = if let Some(ref mut r) = self.connect_ratelimit {
            if r.try_wait().is_ok() {
                debug!("connect tokens available");
                None
            } else {
                Some(())
            }
        } else {
            None
        };

        if wait.is_none() {
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
        } else {
            debug!("delay reconnect {:?}", token);
            self.set_timeout(token);
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

    /// idle connection
    /// - reconnect on failure
    /// - transition to Reading if entire buffer written in one call
    fn idle(&mut self, token: Token) {
        trace!("idle {:?}", token);
        if let Some(s) = self.connections[token].stream() {
            self.register(s, token);
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
                    trace!("switch to established");
                    self.clear_timer(token);
                    self.set_state(token, State::Established);
                    self.idle(token);
                    self.connections[token].clear_failures();
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
        match self.state(token) {
            State::Connecting => {
                self.send_stat(token, Stat::ConnectTimeout);
                self.connections[token].connect_failed();
                self.reconnect(token);
            }
            State::Closed => {
                self.reconnect(token);
            }
            State::Established => error!("timeout for State::Established"),
            State::Reading => {
                self.send_stat(token, Stat::ResponseTimeout);
                self.connections[token].request_failed();
                self.reconnect(token);
            }
            State::Writing => {
                error!("timeout for State::Writing");
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
            self.connections[token].connect_failed();
            self.reconnect(token);
        }
    }

    /// try to send the next request using the given token
    /// - requeue in front if no work to send
    /// - halt: if the connection isn't actually writable
    fn try_send(&mut self, token: Token) {
        if self.connections[token].is_writable() {
            if let Some(work) = self.queue.pop() {
                trace!("send {:?}", token);
                self.write(token, work);
            } else {
                self.ready.push_front(token);
            }
        } else {
            halt!(
                "internal state error. dispatch to non-writable {:?}",
                self.state(token)
            );
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
                if self.connect_timeout.is_none() {
                    self.reconnect(token);
                }
                return;
            } else {
                trace!("connection established {:?}", token);
                self.send_stat(token, Stat::ConnectOk);
                self.clear_timer(token);
                self.set_state(token, State::Writing);
                self.ready.push_back(token);
            }
        } else {
            if UnixReady::from(event.readiness()).is_hup() {
                debug!("server hangup {:?}", token);
                self.reconnect(token);
                return;
            }
            match self.state(token) {
                State::Established => {
                    trace!("ready to write {:?}", token);
                    self.send_stat(token, Stat::SocketRead);
                    self.set_state(token, State::Writing);
                    self.ready.push_back(token);
                }
                State::Reading => {
                    trace!("reading {:?}", token);
                    self.send_stat(token, Stat::SocketRead);
                    self.read(token);
                }
                State::Writing => {
                    trace!("writing {:?}", token);
                    self.send_stat(token, Stat::SocketFlush);
                    self.flush(token);
                }
                _ => {}
            }
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
        let mut events = self
            .events
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
            self.try_send(token);
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
