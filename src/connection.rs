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
extern crate time;

use bytes::{Buf, ByteBuf, MutByteBuf};
use mio::{TryRead, TryWrite};
use mio::tcp::TcpStream;
use std::sync::mpsc;
use std::str;

use client::Client;
use parser::{Parse, ParsedResponse, echo, memcache, ping, redis, thrift};
use state::State;
use stats::{Stat, Status};
use workload::Protocol;

const MEGABYTE: usize = 1024 * 1024;

pub struct Connection {
    pub socket: TcpStream,
    pub token: mio::Token,
    pub state: State,
    buf: Option<ByteBuf>,
    mut_buf: Option<MutByteBuf>,
    last_write: u64,
    stats_tx: mpsc::Sender<Stat>,
    protocol: Protocol,
}

impl Connection {
    pub fn new(socket: TcpStream,
               token: mio::Token,
               stats_tx: mpsc::Sender<Stat>,
               protocol: Protocol,
               tcp_nodelay: bool)
               -> Connection {

        let _ = socket.set_nodelay(tcp_nodelay);

        Connection {
            socket: socket,
            token: token,
            state: State::Writing,
            buf: Some(ByteBuf::none()),
            mut_buf: Some(ByteBuf::mut_with_capacity(4 * MEGABYTE)),
            last_write: time::precise_time_ns(),
            stats_tx: stats_tx,
            protocol: protocol,
        }
    }

    pub fn ready(&mut self,
                 event_loop: &mut mio::EventLoop<Client>,
                 events: mio::EventSet,
                 work: Option<Vec<u8>>) {

        trace!("    connection-state={:?}", self.state);

        match self.state {
            State::Reading => {
                assert!(events.is_readable(),
                        "unexpected events; events={:?}",
                        events);
                let now = time::precise_time_ns();
                let response = self.read(event_loop);
                match response {
                    ParsedResponse::Hit => {
                        let _ = self.stats_tx.send(Stat {
                            start: self.last_write,
                            stop: now,
                            status: Status::Hit,
                        });
                    }
                    ParsedResponse::Ok => {
                        let _ = self.stats_tx.send(Stat {
                            start: self.last_write,
                            stop: now,
                            status: Status::Ok,
                        });
                    }
                    ParsedResponse::Miss => {
                        let _ = self.stats_tx.send(Stat {
                            start: self.last_write,
                            stop: now,
                            status: Status::Miss,
                        });
                    }
                    ParsedResponse::Incomplete => {}
                    ParsedResponse::Unknown => {
                        let _ = self.stats_tx.send(Stat {
                            start: self.last_write,
                            stop: now,
                            status: Status::Closed,
                        });
                    }
                    _ => {
                        let _ = self.stats_tx.send(Stat {
                            start: self.last_write,
                            stop: now,
                            status: Status::Error,
                        });
                        debug!("unexpected response: {:?}", response);
                    }
                }
            }
            State::Writing => {
                assert!(events.is_writable(),
                        "unexpected events; events={:?}",
                        events);
                if let Some(w) = work {
                    let mut buf = match self.mut_buf.take() {
                        Some(b) => b,
                        None => {
                            panic!("no mut_buf to take");
                        }
                    };
                    buf.clear();
                    buf.write_slice(&*w);
                    self.buf = Some(buf.flip());
                    self.write(event_loop)
                } else {
                    panic!("no work");
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn read(&mut self, event_loop: &mut mio::EventLoop<Client>) -> ParsedResponse {

        trace!("read()");

        // response unknown until parsed
        let mut resp = ParsedResponse::Unknown;

        let mut buf = match self.mut_buf.take() {
            Some(b) => b,
            None => {
                panic!("read() no mut_buf");
            }
        };

        match self.socket.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                trace!("read() closed");
                self.state = State::Closed;
            }
            Ok(Some(n)) => {
                // read bytes from connection
                trace!("read() bytes {}", n);

                let buf = buf.flip();

                // protocol dependant parsing
                resp = match self.protocol {
                    Protocol::Echo => echo::Response { response: buf.bytes() }.parse(),
                    Protocol::Thrift => thrift::Response { response: buf.bytes() }.parse(),
                    Protocol::Memcache => {
                        match str::from_utf8(buf.bytes()) {
                            Ok(msg) => memcache::Response { response: msg }.parse(),
                            Err(_) => ParsedResponse::Invalid,
                        }
                    }
                    Protocol::Redis => {
                        match str::from_utf8(buf.bytes()) {
                            Ok(msg) => redis::Response { response: msg }.parse(),
                            Err(_) => ParsedResponse::Invalid,
                        }
                    }
                    Protocol::Ping => {
                        match str::from_utf8(buf.bytes()) {
                            Ok(msg) => ping::Response { response: msg }.parse(),
                            Err(_) => ParsedResponse::Invalid,
                        }
                    }
                    Protocol::Unknown => {
                        panic!("unhandled protocol!");
                    }
                };

                // if incomplete replace the buffer contents, otherwise transition
                match resp {
                    ParsedResponse::Incomplete => {
                        trace!("read() Incomplete");
                        self.mut_buf = Some(buf.resume());
                    }
                    _ => {
                        trace!("read() Complete");
                        self.state = State::Writing;
                        self.mut_buf = Some(buf.flip());
                    }
                }

                self.reregister(event_loop);
            }
            Ok(None) => {
                trace!("read() spurious wake-up");
                self.mut_buf = Some(buf);
                self.reregister(event_loop);
            }
            Err(e) => {
                debug!("server has terminated: {}", e);
                self.state = State::Closed;
            }
        }
        resp
    }

    pub fn write(&mut self, event_loop: &mut mio::EventLoop<Client>) {
        trace!("write()");
        self.state = State::Writing;
        self.last_write = time::precise_time_ns(); // mark time of write
        let mut buf = self.buf.take().unwrap();
        match self.socket.try_write_buf(&mut buf) {
            Ok(Some(_)) => {
                // successful write
                if !buf.has_remaining() {
                    self.state = State::Reading;
                    trace!("switch to read()");
                }
                self.reregister(event_loop);
            }
            Ok(None) => {
                // socket wasn't ready
                self.reregister(event_loop);
            }
            Err(e) => {
                // got some write error, abandon
                debug!("got an error trying to write; err={:?}", e);
                self.state = State::Closed
            }
        }
        self.mut_buf = Some(buf.flip());
    }

    pub fn reregister(&self, event_loop: &mut mio::EventLoop<Client>) {
        event_loop.reregister(&self.socket,
                              self.token,
                              event_set(self.state.clone()),
                              mio::PollOpt::edge())
                  .unwrap();
    }
}

// State to mio EventSet mapping
fn event_set(state: State) -> mio::EventSet {
    match state {
        State::Reading => mio::EventSet::readable(),
        State::Writing => mio::EventSet::writable(),
        _ => mio::EventSet::none(),
    }
}
