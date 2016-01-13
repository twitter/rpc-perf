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
extern crate parser;

use client::Client;
use bytes::Take;
use std::io::Cursor;
use mio::{TryRead, TryWrite};
use mio::tcp::*;
use state::State;
use stats::*;
use workload::Protocol;
use std::sync::mpsc;

use parser::*;

pub struct Connection {
    pub socket: TcpStream,
    pub token: mio::Token,
    pub state: State,
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

        let msg: Vec<u8> = Vec::new();
        let len = 0;
        let buf = Cursor::new(msg);
        let _ = socket.set_nodelay(tcp_nodelay);

        Connection {
            socket: socket,
            token: token,
            state: State::Writing(Take::new(buf, len)),
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
            State::Reading(..) => {
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
            State::Writing(..) => {
                assert!(events.is_writable(),
                        "unexpected events; events={:?}",
                        events);
                assert!(work.is_some());
                self.write(event_loop, work.unwrap())
            }
            _ => unimplemented!(),
        }
    }

    pub fn read(&mut self, event_loop: &mut mio::EventLoop<Client>) -> ParsedResponse {

        // response unknown until parsed
        let mut resp = ParsedResponse::Unknown;

        match self.socket.try_read_buf(self.state.mut_read_buf()) {
            Ok(Some(0)) => {
                // read 0 bytes
                // socket is either closed or half shutdown
                // attempt to write any buffered data
                trace!("    read 0 bytes from server; buffered={}",
                       self.state.read_buf().len());

                match self.state.read_buf().len() {
                    n if n > 0 => {
                        // if any data in buffer, switch to write
                        let _ = self.state.transition_to_writing();

                        self.reregister(event_loop);
                    }
                    _ => self.state = State::Closed,
                }
            }
            Ok(Some(n)) => {
                // read some bytes
                trace!("read {} bytes", n);

                // parse the response
                resp = self.state.try_transition_to_writing(self.protocol);

                self.reregister(event_loop);
            }
            Ok(None) => {
                self.reregister(event_loop);
            }
            Err(e) => {
                debug!("server has terminated: {}", e);
                self.state = State::Closed
            }
        }
        resp
    }

    pub fn write(&mut self, event_loop: &mut mio::EventLoop<Client>, msg: Vec<u8>) {
        let len = msg.len();
        let buf = Cursor::new(msg);
        self.state = State::Writing(Take::new(buf, len));
        self.last_write = time::precise_time_ns(); // mark time of write
        match self.socket.try_write_buf(self.state.mut_write_buf()) {
            Ok(Some(_)) => {
                // successful write
                self.state.try_transition_to_reading(self.protocol);
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
    }

    pub fn reregister(&self, event_loop: &mut mio::EventLoop<Client>) {
        event_loop.reregister(&self.socket,
                              self.token,
                              self.state.event_set(),
                              mio::PollOpt::edge())
                  .unwrap();
    }
}
