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
extern crate parser;

use bytes::{Buf, Take};
use std::io::Cursor;
use std::mem;
use parser::*;
use workload::Protocol;

// The current state of the client connection
#[derive(Debug)]
pub enum State {
    Reading(Vec<u8>), // reading from network
    Writing(Take<Cursor<Vec<u8>>>), // writing to network
    Closed,
}

impl State {
    pub fn mut_read_buf(&mut self) -> &mut Vec<u8> {
        match *self {
            State::Reading(ref mut buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn read_buf(&self) -> &[u8] {
        match *self {
            State::Reading(ref buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn read_buf_vec(&self) -> Vec<u8> {
        match *self {
            State::Reading(ref buf) => buf.clone(),
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn write_buf(&self) -> &Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    pub fn mut_write_buf(&mut self) -> &mut Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref mut buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    // if the response is complete, transition to writing
    pub fn try_transition_to_writing(&mut self, protocol: Protocol) -> ParsedResponse {
        match self.read_buf().last() {
            // all complete responses end in '\n'
            Some(&c) if c == b'\n' => {
                // wrap in a scope to work around borrow checker
                {
                    let resp: ParsedResponse;

                    let raw = mem::replace(self, State::Closed).unwrap_read_buf();

                        // protocol dependant parsing
                        match protocol {
                            Protocol::Echo => {
                                resp = echo::Response { response: raw.clone() }.parse();
                            }
                            Protocol::Memcache => {
                                match String::from_utf8(raw.clone()) {
                                    Ok(msg) => {
                                        resp = memcache::Response { response: msg.clone() }.parse();
                                    }
                                    Err(_) => {
                                        resp = ParsedResponse::Invalid;
                                    }
                                }
                                //resp = memcache::Response { response: msg.clone() }.parse();
                            }
                            Protocol::Redis => {
                                match String::from_utf8(raw.clone()) {
                                    Ok(msg) => {
                                        resp = redis::Response { response: msg.clone() }.parse();
                                    }
                                    Err(_) => {
                                        resp = ParsedResponse::Invalid;
                                    }
                                }
                                //resp = redis::Response { response: msg.clone() }.parse();
                            }
                            Protocol::Ping => {
                                //resp = ping::Response { response: msg.clone() }.parse();
                                match String::from_utf8(raw.clone()) {
                                    Ok(msg) => {
                                        resp = ping::Response { response: msg.clone() }.parse();
                                    }
                                    Err(_) => {
                                        resp = ParsedResponse::Invalid;
                                    }
                                }
                            }
                            Protocol::Unknown => {
                                panic!("unhandled protocol!");
                            }
                        }

                        // if incomplete replace the buffer contents, otherwise transition
                        match resp {
                            ParsedResponse::Incomplete => {
                                mem::replace(self, State::Reading(raw));
                                return resp;
                            }
                            _ => { }
                        }
                    

                    self.transition_to_writing();
                    return resp;
                }
            }
            _ => {}
        }
        return ParsedResponse::Incomplete;
    }

    // clear the current buffer and transition state
    pub fn transition_to_writing(&mut self) -> bool {
        let buf: Vec<u8> = Vec::new();

        let buf = Cursor::new(buf);

        *self = State::Writing(Take::new(buf, 0));

        return true;
    }

    // if the write buffer has emptied, transition to reading
    pub fn try_transition_to_reading(&mut self, protocol: Protocol) {
        if !self.write_buf().has_remaining() {
            let cursor = mem::replace(self, State::Closed).unwrap_write_buf().into_inner();

            let pos = cursor.position();
            let mut buf = cursor.into_inner();

            // drop all data that has been written to the client
            drain_to(&mut buf, pos as usize);

            *self = State::Reading(buf);

            // there may already be another response to read
            self.try_transition_to_writing(protocol);
        }
    }

    // State to mio EventSet mapping
    pub fn event_set(&self) -> mio::EventSet {
        match *self {
            State::Reading(..) => mio::EventSet::readable(),
            State::Writing(..) => mio::EventSet::writable(),
            _ => mio::EventSet::none(),
        }
    }

    pub fn unwrap_read_buf(self) -> Vec<u8> {
        match self {
            State::Reading(buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn unwrap_write_buf(self) -> Take<Cursor<Vec<u8>>> {
        match self {
            State::Writing(buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }
}

fn drain_to(vec: &mut Vec<u8>, count: usize) {
    // A very inefficient implementation. A better implementation could be
    // built using `Vec::drain()`, but the API is currently unstable.
    for _ in 0..count {
        vec.remove(0);
    }
}
