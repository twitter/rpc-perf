//  Copyright 2019 Twitter, Inc
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

mod common;
mod plain_client;
#[cfg(feature = "tls")]
mod tls_client;

use crate::codec::*;
use crate::stats::SimpleRecorder;
use rand::rngs::ThreadRng;

pub use crate::client::common::Common;
pub use crate::client::plain_client::PlainClient;
#[cfg(feature = "tls")]
pub use crate::client::tls_client::TLSClient;
use crate::session::*;
use crate::stats::Stat;
use crate::*;

use mio::unix::UnixReady;
use mio::{Event, Events, Poll, PollOpt, Ready, Token};
use ratelimiter::Ratelimiter;

use std::net::SocketAddr;
use std::time::Duration;

pub const SECOND: usize = 1_000_000_000;
pub const MILLISECOND: usize = 1_000_000;
pub const MICROSECOND: usize = 1_000;

pub trait Client: Send {
    // configuration
    fn add_endpoint(&mut self, server: &SocketAddr);
    fn set_connect_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.common_mut().set_connect_ratelimit(ratelimiter)
    }
    fn set_connect_timeout(&mut self, microseconds: usize) {
        self.common_mut().set_connect_timeout(microseconds)
    }
    fn set_poolsize(&mut self, connections: usize) {
        self.common_mut().set_poolsize(connections);
    }
    fn poolsize(&self) -> usize {
        self.common().poolsize()
    }
    fn set_tcp_nodelay(&mut self, nodelay: bool) {
        self.common_mut().set_tcp_nodelay(nodelay);
    }
    fn tcp_nodelay(&self) -> bool {
        self.common().tcp_nodelay()
    }
    fn set_request_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.common_mut().set_request_ratelimit(ratelimiter)
    }
    fn set_request_timeout(&mut self, microseconds: usize) {
        self.common_mut().set_request_timeout(microseconds)
    }
    fn set_stats(&mut self, recorder: SimpleRecorder) {
        self.common_mut().set_stats(recorder);
    }
    fn set_close_rate(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.common_mut().set_close_rate(ratelimiter);
    }
    fn set_soft_timeout(&mut self, enabled: bool) {
        self.common_mut().set_soft_timeout(enabled);
    }
    fn soft_timeout(&self) -> bool {
        self.common().soft_timeout()
    }

    // implementation specific
    fn common(&self) -> &Common;
    fn common_mut(&mut self) -> &mut Common;
    fn do_timeouts(&mut self) {
        let timers = self.common_mut().get_timers();
        if !timers.is_empty() {
            debug!("Processing: {} timeouts", timers.len());
        }
        for token in timers {
            self.do_timeout(token);
        }
    }
    fn does_negotiate(&self) -> bool;
    fn session(&self, token: Token) -> &Session;
    fn session_mut(&mut self, token: Token) -> &mut Session;

    fn prepare_request(&mut self, token: Token, rng: &mut ThreadRng);

    // client id
    fn id(&self) -> usize {
        self.common().id()
    }

    // stats helpers
    fn stat_increment(&self, label: Stat) {
        self.common().stat_increment(label);
    }

    fn stat_interval(&self, label: Stat, start: u64, stop: u64) {
        self.common().stat_interval(label, start, stop);
    }

    fn heatmap_increment(&self, start: u64, stop: u64) {
        self.common().heatmap_increment(start, stop);
    }

    // connect queue
    fn connect_pending(&self) -> usize {
        self.common().connect_pending()
    }
    fn connect_dequeue(&mut self) -> Option<Token> {
        self.common_mut().connect_dequeue()
    }
    fn connect_enqueue(&mut self, token: Token) {
        debug!("connect enqueue");
        self.common_mut().connect_enqueue(token);
    }
    fn connect_requeue(&mut self, token: Token) {
        debug!("connect requeue");
        self.common_mut().connect_requeue(token);
    }
    fn connect_shuffle(&mut self) {
        debug!("shuffle connect queue");
        self.common_mut().connect_shuffle();
    }

    // ready queue
    fn ready_dequeue(&mut self) -> Option<Token> {
        self.common_mut().ready_dequeue()
    }
    fn ready_enqueue(&mut self, token: Token) {
        self.common_mut().ready_enqueue(token);
    }
    fn ready_requeue(&mut self, token: Token) {
        self.common_mut().ready_requeue(token);
    }

    // token registration
    fn deregister(&mut self, token: Token) -> Result<(), std::io::Error> {
        self.session(token).deregister(self.event_loop())
    }
    fn register(&mut self, token: Token) -> Result<(), std::io::Error> {
        self.session(token).register(
            token,
            self.event_loop(),
            self.event_set(token),
            self.poll_opt(token),
        )
    }
    fn reregister(&mut self, token: Token) -> Result<(), std::io::Error> {
        self.session(token).reregister(
            token,
            self.event_loop(),
            self.event_set(token),
            self.poll_opt(token),
        )
    }

    // write helper
    fn do_write(&mut self, token: Token) {
        self.set_state(token, State::Writing);
    }

    // mio::Ready
    fn event_set(&self, token: Token) -> mio::Ready {
        match self.session(token).state() {
            State::Closed => Ready::empty(),
            State::Connecting => Ready::writable() | UnixReady::hup(),
            State::Established => Ready::empty() | UnixReady::hup(),
            State::Writing => Ready::writable() | UnixReady::hup(),
            State::Reading => Ready::readable() | UnixReady::hup(),
            State::Negotiating => Ready::readable() | Ready::writable() | UnixReady::hup(),
        }
    }

    // mio::PollOpt
    fn poll_opt(&self, _token: Token) -> PollOpt {
        PollOpt::edge() | PollOpt::oneshot()
    }

    // mio::Events
    fn set_events(&mut self, events: Option<Events>) {
        self.common_mut().set_events(events)
    }
    fn take_events(&mut self) -> Option<Events> {
        self.common_mut().take_events()
    }

    // mio::Poll
    fn event_loop(&self) -> &Poll {
        self.common().event_loop()
    }

    // timeout handling
    fn clear_timeout(&mut self, token: Token) {
        self.common_mut().cancel_timer(token);
    }
    fn set_timeout(&mut self, token: Token, microseconds: usize) {
        self.common_mut().add_timer(token, microseconds);
    }
    fn do_timeout(&mut self, token: Token) {
        let state = self.session(token).state();
        debug!("timeout on client {} {:?} {:?}", self.id(), token, state);
        match state {
            State::Connecting | State::Negotiating => {
                self.stat_increment(Stat::ConnectionsTimeout);
                if !self.soft_timeout() {
                    self.set_state(token, State::Closed);
                }
            }
            State::Closed | State::Established | State::Writing => {
                debug!("ignore timeout");
            }
            State::Reading => {
                self.stat_increment(Stat::RequestsTimeout);
                if !self.soft_timeout() {
                    self.set_state(token, State::Closed);
                }
            }
        }
        self.clear_timeout(token);
    }

    fn do_close(&mut self, token: Token) {
        trace!(
            "do_close on client {} {:?} {:?}",
            self.id(),
            token,
            self.state(token)
        );
        let _ = self.deregister(token);
        self.clear_timeout(token);
        self.set_session_state(token, State::Closed);
        self.connect_enqueue(token);
    }

    fn do_established(&mut self, token: Token) {
        let _ = self.deregister(token);
        self.clear_timeout(token);
        self.set_session_state(token, State::Established);
        self.ready_enqueue(token);
    }

    fn do_negotiating(&mut self, token: Token) {
        debug!("begin tls negotiation");
        self.set_session_state(token, State::Negotiating);
        if self.reregister(token).is_err() {
            let _ = self.register(token);
        }
    }

    fn do_connecting(&mut self, token: Token) {
        self.stat_increment(Stat::ConnectionsTotal);
        if self.session_mut(token).connect().is_ok() {
            trace!("socket opened: client {} {:?}", self.id(), token);
            self.session_mut(token)
                .set_timestamp(Some(time::precise_time_ns()));
            self.set_session_state(token, State::Connecting);
            // TODO: use a configurable timeout value w/ policy here
            self.set_timeout(token, self.common().connect_timeout());
            if self.register(token).is_err() {
                fatal!("Error registering: {:?}", State::Connecting);
            }
        } else {
            debug!("socket error: client {} {:?}", self.id(), token);
            self.stat_increment(Stat::ConnectionsError);
            self.stat_increment(Stat::ConnectionsClosed);
            self.connect_enqueue(token);
        }
    }

    fn do_writing(&mut self, token: Token) {
        self.set_session_state(token, State::Writing);
        if self.reregister(token).is_err() {
            let _ = self.register(token);
        }
    }

    fn do_reading(&mut self, token: Token) {
        self.set_session_state(token, State::Reading);
        if self.reregister(token).is_err() {
            let _ = self.register(token);
        }
        self.set_timeout(token, self.common().request_timeout());
    }

    // Ratelimiter helpers
    fn try_connect_wait(&self) -> Result<(), ()> {
        self.common().try_connect_wait()
    }

    fn try_request_wait(&self) -> Result<(), ()> {
        self.common().try_request_wait()
    }

    fn should_close(&self) -> bool {
        self.common().should_close()
    }

    // protocol helpers
    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        self.common().decode(buf)
    }

    // state machine
    fn set_state(&mut self, token: Token, target: State) {
        let state = self.state(token);
        match state {
            State::Connecting => match target {
                State::Established => {
                    trace!("connection established {:?}", token);
                    if let Some(t0) = self.session(token).timestamp() {
                        self.stat_interval(Stat::ConnectionsOpened, t0, time::precise_time_ns());
                    }
                    self.do_established(token);
                }
                State::Closed => {
                    self.do_close(token);
                }
                State::Negotiating => {
                    self.do_negotiating(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
            State::Closed => match target {
                State::Connecting => {
                    self.do_connecting(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
            State::Established => match target {
                State::Closed => {
                    self.stat_increment(Stat::ConnectionsClosed);
                    self.do_close(token);
                }
                State::Writing => {
                    trace!("writing to established connection");
                    self.do_writing(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
            State::Negotiating => match target {
                State::Established => {
                    debug!("session established");
                    if let Some(t0) = self.session(token).timestamp() {
                        self.stat_interval(Stat::ConnectionsOpened, t0, time::precise_time_ns());
                    }
                    self.do_established(token);
                }
                State::Closed => {
                    self.do_close(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
            State::Writing => match target {
                State::Reading => {
                    trace!("request sent, switch to reading");
                    self.stat_increment(Stat::RequestsDequeued);
                    self.do_reading(token);
                }
                State::Closed => {
                    self.stat_increment(Stat::RequestsError);
                    self.stat_increment(Stat::ConnectionsClosed);
                    self.do_close(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
            State::Reading => match target {
                State::Closed => {
                    self.stat_increment(Stat::RequestsError);
                    self.stat_increment(Stat::ConnectionsClosed);
                    self.do_close(token);
                }
                State::Established => {
                    trace!("response complete, switch to Established");
                    self.stat_increment(Stat::ResponsesTotal);
                    self.do_established(token);
                }
                _ => {
                    error!("Unhandled state transition: {:?} -> {:?}", state, target);
                }
            },
        }
    }

    fn set_session_state(&mut self, token: Token, state: State) {
        self.session_mut(token).set_state(state)
    }

    fn handle_negotiating(&mut self, token: Token, event: Event) {
        trace!("Got event on Negotiating connection");
        if UnixReady::from(event.readiness()).is_hup() {
            trace!("hangup on connect {:?}", token);
            self.stat_increment(Stat::ConnectionsError);
            self.stat_increment(Stat::ConnectionsServerClosed);
            self.set_state(token, State::Closed);
        } else {
            let read_result = if event.readiness().is_readable() {
                self.session_mut(token).session_read()
            } else {
                Ok(())
            };
            let write_result = if event.readiness().is_writable() {
                self.session_mut(token).session_flush()
            } else {
                Ok(())
            };
            if self.session(token).is_handshaking() {
                self.reregister(token).expect("failed to register");
            } else if read_result.is_ok() && write_result.is_ok() {
                self.set_state(token, State::Established);
            } else {
                self.set_state(token, State::Closed);
            }
        }
    }

    fn handle_connecting(&mut self, token: Token, event: Event) {
        trace!("Got event on connecting connection");
        if UnixReady::from(event.readiness()).is_hup() {
            trace!("hangup on connect {:?}", token);
            self.stat_increment(Stat::ConnectionsError);
            self.stat_increment(Stat::ConnectionsServerClosed);
            self.set_state(token, State::Closed);
        } else if self.does_negotiate() {
            self.set_state(token, State::Negotiating);
        } else {
            self.set_state(token, State::Established);
        }
    }

    fn handle_established(&mut self, token: Token, event: Event) {
        trace!("Got event on established connection");
        if self.should_close() {
            self.stat_increment(Stat::ConnectionsClientClosed);
            self.set_state(token, State::Closed);
        } else if UnixReady::from(event.readiness()).is_hup() {
            trace!("hangup on established {:?}", token);
            self.stat_increment(Stat::ConnectionsServerClosed);
            self.set_state(token, State::Closed);
        } else {
            self.set_state(token, State::Established);
        }
    }

    fn handle_reading(&mut self, token: Token, event: Event) {
        trace!(
            "Got event on reading connection: client {} {:?} {:?}",
            self.id(),
            token,
            event
        );
        if self.should_close() {
            self.stat_increment(Stat::ConnectionsClientClosed);
            self.set_state(token, State::Closed);
        } else if UnixReady::from(event.readiness()).is_hup() {
            self.stat_increment(Stat::ConnectionsServerClosed);
            self.set_state(token, State::Closed);
        } else {
            let _result = self.session_mut(token).read_to();
            let buf = self.session(token).read_buf();
            trace!("buffer: {:?}", buf);
            let len = buf.len();
            match len {
                0 => {
                    trace!("EOF on read");
                    self.stat_increment(Stat::ConnectionsServerClosed);
                    self.set_state(token, State::Closed);
                }
                n => {
                    trace!("Got a response: {} bytes", n);
                    let t1 = time::precise_time_ns();
                    let parsed = self.decode(buf);
                    match parsed {
                        Ok(Response::Ok) => {
                            self.stat_increment(Stat::ResponsesOk);
                        }
                        Ok(Response::Hit) => {
                            self.stat_increment(Stat::ResponsesOk);
                            self.stat_increment(Stat::ResponsesHit);
                        }
                        Ok(Response::Miss) => {
                            self.stat_increment(Stat::ResponsesOk);
                            self.stat_increment(Stat::ResponsesMiss);
                        }
                        Err(Error::Incomplete) => {
                            if self.reregister(token).is_err() {
                                let _ = self.register(token);
                            }
                        }
                        Err(_) => {
                            self.stat_increment(Stat::ResponsesError);
                        }
                        Ok(_) => {
                            self.stat_increment(Stat::ResponsesOk);
                        }
                    };

                    if parsed != Err(Error::Incomplete) {
                        if let Some(t0) = self.session(token).timestamp() {
                            self.stat_interval(Stat::ResponsesTotal, t0, t1);
                            self.heatmap_increment(t0, t1);
                        }
                        trace!("switch to established");
                        self.clear_timeout(token);
                        self.set_state(token, State::Established);
                        self.session_mut(token).clear_buffer();
                    }
                }
            }
        }
    }

    fn handle_writing(&mut self, token: Token, event: Event) {
        trace!("Got event on writing connection");
        if self.should_close() {
            self.stat_increment(Stat::ConnectionsClientClosed);
            self.set_state(token, State::Closed);
        } else if UnixReady::from(event.readiness()).is_hup() {
            self.stat_increment(Stat::ConnectionsServerClosed);
            self.set_state(token, State::Closed);
        } else {
            match self.session_mut(token).flush() {
                Ok(_) => {
                    self.set_state(token, State::Reading);
                }
                Err(_) => {
                    // incomplete write, register again in same state
                    self.reregister(token).expect("failed to register");
                }
            }
        }
    }

    // mio::Event handler
    fn ready(&mut self, token: Token, event: Event) {
        let state = self.state(token);
        trace!("ready: {:?} {:?} {:?}", token, state, event);
        match state {
            State::Closed => {
                error!("Got event on closed connection");
            }
            State::Negotiating => self.handle_negotiating(token, event),
            State::Connecting => self.handle_connecting(token, event),
            State::Established => self.handle_established(token, event),
            State::Reading => self.handle_reading(token, event),
            State::Writing => self.handle_writing(token, event),
        }
    }

    fn state(&self, token: Token) -> State {
        self.session(token).state()
    }

    // main function
    fn run(&mut self, rng: &mut ThreadRng) {
        // handle any timeouts
        self.do_timeouts();

        // handle any events
        self.do_events();

        // send requests
        self.do_requests(rng);

        // close connections
        self.do_client_terminations();

        // open connections
        self.do_connects();
    }

    // handle events
    fn do_events(&mut self) {
        let mut events = self
            .take_events()
            .unwrap_or_else(|| Events::with_capacity(1024));
        self.event_loop()
            .poll(&mut events, Some(Duration::from_millis(1)))
            .unwrap();

        for event in events.iter() {
            let token = event.token();
            self.ready(token, event);
        }

        self.set_events(Some(events));
    }

    // disbatch requests
    fn do_requests(&mut self, rng: &mut ThreadRng) {
        // send a single request
        if let Some(token) = self.ready_dequeue() {
            if self.try_request_wait().is_ok() {
                self.prepare_request(token, rng);
                self.do_write(token);
                self.stat_increment(Stat::RequestsEnqueued);
            } else {
                self.ready_requeue(token);
            }
        }
    }

    // client connection terminations connection
    fn do_client_terminations(&mut self) {
        if self.should_close() {
            if let Some(token) = self.ready_dequeue() {
                trace!("closing");
                self.stat_increment(Stat::ConnectionsClientClosed);
                self.do_close(token);
            }
        }
    }

    // establish connections
    fn do_connects(&mut self) {
        let needed = self.connect_pending();
        if needed > 0 {
            debug!("pending connections: client {} {}", self.id(), needed);
        }

        // do a single connect
        if let Some(token) = self.connect_dequeue() {
            if self.try_connect_wait().is_ok() {
                trace!("connecting...");
                self.set_state(token, State::Connecting);
            } else {
                debug!("Ratelimiting connect");
                self.connect_requeue(token);
            }
        }
    }
}
