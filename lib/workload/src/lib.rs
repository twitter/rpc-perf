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

#![crate_type = "lib"]
#![crate_name = "workload"]

extern crate mpmc;
extern crate request;
extern crate ratelimit;
extern crate time;
extern crate rand;
extern crate shuteye;

use mpmc::Queue as BoundedQueue;
use rand::{thread_rng, Rng};

use ratelimit::Ratelimit;
use request::{echo, memcache, ping, redis, thrift};

const ONE_SECOND: u64 = 1_000_000_000;
const BUCKET_SIZE: usize = 10_000;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Protocol {
    Echo,
    Memcache,
    Ping,
    Redis,
    Thrift,
    Unknown,
}

impl Protocol {
    pub fn new(protocol: &str) -> Result<Protocol, ()> {
        match &*protocol {
            "echo" => {
                return Ok(Protocol::Echo);
            }
            "memcache" => {
                return Ok(Protocol::Memcache);
            }
            "ping" => {
                return Ok(Protocol::Ping);
            }
            "redis" => {
                return Ok(Protocol::Redis);
            }
            "thrift" => {
                return Ok(Protocol::Thrift);
            }
            _ => {
                return Err(());
            }
        }
    }
}

pub struct Hotkey {
    id: usize,
    protocol: Protocol,
    command: String,
    ratelimit: Ratelimit,
    rate: u64,
    queue: BoundedQueue<Vec<u8>>,
    length: usize,
    hit: bool,
    flush: bool,
}

fn rate_to_interval(rate: u64) -> u64 {
    if rate == 0 {
        return 0;
    }
    let interval = ONE_SECOND / rate;
    if interval < 1 {
        return 0;
    }
    interval
}

impl Hotkey {
    pub fn new(id: usize,
               protocol: String,
               command: String,
               length: usize,
               rate: u64,
               queue: BoundedQueue<Vec<u8>>,
               quantum: usize,
               hit: bool,
               flush: bool)
               -> Result<Hotkey, &'static str> {

        let cmd = command.clone();
        let proto: Protocol;

        match Protocol::new(&protocol) {
            Ok(p) => {
                proto = p;
            }
            Err(..) => {
                panic!("Bad protocol: {}", protocol);
            }
        }

        let interval = rate_to_interval(rate);
        match Ratelimit::new(BUCKET_SIZE, time::precise_time_ns(), interval, quantum) {
            Some(r) => {
                return Ok(Hotkey {
                    id: id,
                    protocol: proto,
                    command: cmd,
                    length: length,
                    rate: rate,
                    ratelimit: r,
                    queue: queue,
                    hit: hit,
                    flush: flush,
                });
            }
            None => {
                return Err("Ratelimit::new() returned None");
            }
        }
    }

    fn random_value(length: usize) -> String {
        thread_rng().gen_ascii_chars().take(length).collect()
    }

    pub fn run(&mut self) {
        // calculate the query

        let query: Vec<u8>;

        let key = format!("{}", self.id);

        let value = Self::random_value(self.length);

        match self.protocol {
            Protocol::Memcache => {
                if self.flush {
                    let flush = memcache::flush_all().into_bytes();
                    let _ = self.queue.push(flush);
                    shuteye::sleep(shuteye::Timespec::from_nano(1_000_000 as i64).unwrap());
                }
                match &*self.command {
                    "set" => {
                        query = memcache::set(&key, &*value, None, None).into_bytes();
                    }
                    "get" => {
                        if self.hit {
                            let prepare = memcache::set(&key, &*value, None, None).into_bytes();
                            for _ in 0..5 {
                                let _ = self.queue.push(prepare.clone());
                            }
                            shuteye::sleep(shuteye::Timespec::from_nano(1_000_000 as i64).unwrap());
                        }
                        query = memcache::get(&key).into_bytes();
                    }
                    _ => {
                        panic!("unknown command: {} for protocol: {:?}",
                               self.command,
                               self.protocol);
                    }
                }
            }
            Protocol::Ping => {
                match &*self.command {
                    "ping" => {
                        query = ping::ping().into_bytes();
                    }
                    _ => {
                        panic!("unknown command: {} for protocol: {:?}",
                               self.command,
                               self.protocol);
                    }
                }
            }
            Protocol::Redis => {
                if self.flush {
                    let flush = redis::flushall().into_bytes();
                    let _ = self.queue.push(flush);
                }
                match &*self.command {
                    "set" => {
                        query = redis::set(&key, &*value).into_bytes();
                    }
                    "get" => {
                        if self.hit {
                            let prepare = redis::set(&key, &*value).into_bytes();
                            let _ = self.queue.push(prepare);
                        }
                        query = redis::get(&key).into_bytes();
                    }
                    "hset" => {
                        query = redis::hset(&key, "key", &*value).into_bytes();
                    }
                    "hget" => {
                        if self.hit {
                            let prepare = redis::hset(&key, "key", &*value).into_bytes();
                            let _ = self.queue.push(prepare);
                        }
                        query = redis::hget(&key, "key").into_bytes();
                    }
                    _ => {
                        panic!("unknown command: {} for protocol: {:?}",
                               self.command,
                               self.protocol);
                    }
                }
            }
            Protocol::Echo => {
                match &*self.command {
                    "echo" => {
                        query = echo::echo(&*value);
                    }
                    _ => {
                        panic!("unknown command: {} for protocol: {:?}",
                               self.command,
                               self.protocol);
                    }
                }
            }
            Protocol::Thrift => {
                match &*self.command {
                    "ping" => {
                        query = thrift::ping();
                    }
                    _ => {
                        panic!("unknown command: {} for protocol: {:?}",
                               self.command,
                               self.protocol);
                    }
                }
            }
            Protocol::Unknown => panic!("unknown protocol"),
        }

        // critical sections
        if self.rate == 0 {
            loop {
                let _ = self.queue.push(query.clone());
            }
        } else {
            loop {
                self.ratelimit.block(1);
                let _ = self.queue.push(query.clone());
            }
        }
    }
}
