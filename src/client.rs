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

use mio::util::Slab;
use mpmc::Queue as BoundedQueue;

use connection::Connection;
use state::State;

const MAX_CONNECTIONS: usize = 1024;

pub struct Client {
    pub connections: Slab<Connection>,
    work_rx: BoundedQueue<Vec<u8>>,
}

impl Client {
    pub fn new(work_rx: BoundedQueue<Vec<u8>>) -> Client {
        let connections = Slab::new_starting_at(mio::Token(0), MAX_CONNECTIONS);

        Client {
            connections: connections,
            work_rx: work_rx,
        }
    }
}

impl mio::Handler for Client {
    type Timeout = (); // timeouts not used
    type Message = (); // cross-thread notifications not used

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<Client>,
             token: mio::Token,
             events: mio::EventSet) {
        trace!("socket ready: token={:?} events={:?}", token, events);

        match self.connections[token].state {
            State::Closed => {
                let _ = self.connections.remove(token);
            }
            State::Reading => {
                self.connections[token].ready(event_loop, events, None);
                self.connections[token].reregister(event_loop);
            }
            State::Writing => {
                match self.work_rx.pop() {
                    Some(work) => {
                        trace!("sending: {:?}", work);
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
}
