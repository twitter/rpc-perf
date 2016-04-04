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

extern crate mpmc;
extern crate pad;
extern crate rand;
extern crate ratelimit;
extern crate rpcperf_request as request;
extern crate shuteye;
extern crate time;

const ONE_SECOND: u64 = 1_000_000_000;
pub const BUCKET_SIZE: usize = 10_000;

use mpmc::Queue as BoundedQueue;
use pad::{PadStr, Alignment};
use rand::{thread_rng, Rng};
use ratelimit::Ratelimit;
use request::{echo, memcache, ping, redis, thrift};
use std::str;
use request::thrift::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Protocol {
    Echo,
    Memcache,
    Ping,
    Redis,
    Thrift,
    Unknown,
}

impl Protocol {
    pub fn new(protocol: &str) -> Result<Protocol, &'static str> {
        match &*protocol {
            "echo" => Ok(Protocol::Echo),
            "memcache" => Ok(Protocol::Memcache),
            "ping" => Ok(Protocol::Ping),
            "redis" => Ok(Protocol::Redis),
            "thrift" => Ok(Protocol::Thrift),
            _ => Err("unknown protocol"),
        }
    }
}

pub fn rate_to_interval(rate: u64) -> u64 {
    if rate == 0 {
        return 0;
    }
    let interval = ONE_SECOND / rate;
    if interval < 1 {
        return 0;
    }
    interval
}

fn random_string(size: usize) -> String {
    thread_rng().gen_ascii_chars().take(size).collect()
}

fn seeded_string(size: usize, seed: usize) -> String {
    let s = format!("{}", seed);
    s.pad(size, '0', Alignment::Right, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Stop,
    Void,
    Bool,
    Byte,
    Double,
    Int16,
    Int32,
    Int64,
    String,
    Struct,
    Map,
    Set,
    List(String),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    Random,
    Static,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub id: Option<i16>,
    pub ptype: Type,
    pub seed: usize,
    pub size: usize,
    pub style: Style,
    pub regenerate: bool,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Stop,
    Void,
    Bool(bool),
    Byte(u8),
    Double(f64),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    String(String),
    Struct,
    Map,
    Set,
    List(String, i32),
    None,
}

impl Default for Value {
    fn default() -> Value {
        Value::None
    }
}

impl Default for Parameter {
    fn default() -> Parameter {
        Parameter {
            id: None,
            ptype: Type::None,
            seed: 0,
            size: 1,
            style: Style::Static,
            regenerate: false,
            value: Default::default(),
        }
    }
}

pub enum Preparation {
    Flush,
    Hit,
}

pub struct Workload {
    protocol: Protocol,
    command: String,
    rate: u64,
    ratelimit: Ratelimit,
    queue: BoundedQueue<Vec<u8>>,
    parameters: Vec<Parameter>,
    values: Vec<Vec<u8>>,
}

impl Workload {
    pub fn new(protocol: Protocol,
               command: String,
               rate: Option<u64>,
               queue: BoundedQueue<Vec<u8>>)
               -> Result<Workload, &'static str> {
        let r = rate.unwrap_or(0);
        let i = rate_to_interval(r);
        let ratelimit = match Ratelimit::new(BUCKET_SIZE, time::precise_time_ns(), i, 1) {
            Some(r) => r,
            None => {
                return Err("Ratelimit initialization failed!");
            }
        };
        Ok(Workload {
            protocol: protocol,
            command: command,
            rate: rate.unwrap_or(0),
            ratelimit: ratelimit,
            queue: queue,
            parameters: Vec::<Parameter>::new(),
            values: Vec::<Vec<u8>>::new(),
        })
    }

    fn generate_values(&mut self, force: bool) {
        for i in 0..self.parameters.len() {
            match self.parameters[i].style {
                Style::Random => {
                    if self.parameters[i].regenerate || force {
                        self.parameters[i].value = match self.parameters[i].ptype {
                            Type::Stop => Value::Stop,
                            Type::Void => Value::Void,
                            Type::Bool => Value::Bool(rand::random::<bool>()),
                            Type::Int16 => Value::Int16(rand::random::<i16>()),
                            Type::Int32 => Value::Int32(rand::random::<i32>()),
                            Type::Int64 => Value::Int64(rand::random::<i64>()),
                            Type::Struct => Value::Struct,
                            Type::String => Value::String(random_string(self.parameters[i].size)),
                            _ => {
                                self.values[i] = random_string(self.parameters[i].size)
                                                     .into_bytes();
                                Value::None
                            }
                        }
                    }
                }
                Style::Static => {
                    self.parameters[i].value = match self.parameters[i].ptype.clone() {
                        Type::Int16 => Value::Int16(self.parameters[i].seed as i16),
                        Type::Int32 => Value::Int32(self.parameters[i].seed as i32),
                        Type::Int64 => Value::Int64(self.parameters[i].seed as i64),
                        Type::List(ttype) => Value::List(ttype.clone(), self.parameters[i].seed as i32),
                        Type::Stop => Value::Stop,
                        Type::Struct => Value::Struct,
                        _ => {
                            self.values[i] = seeded_string(self.parameters[i].size,
                                                           self.parameters[i].seed)
                                                 .into_bytes();
                            Value::None
                        }
                    }
                }
            }
        }
    }

    pub fn add_param(&mut self, parameter: Parameter) {
        self.parameters.push(parameter);
        self.values.push(Vec::<u8>::new());
    }

    pub fn prepare(&mut self, preparation: Preparation) {
        match preparation {
            Preparation::Flush => {
                match self.protocol {
                    Protocol::Memcache => {
                        let _ = self.queue.push(memcache::flush_all().into_bytes());
                    }
                    Protocol::Redis => {
                        let _ = self.queue.push(redis::flushall().into_bytes());
                    }
                    _ => {}
                }
            }
            Preparation::Hit => {
                match &*self.command {
                    "get" => {
                        match self.protocol {
                            Protocol::Memcache => {
                                if self.values.len() < 2 {
                                    self.values.push(Default::default());
                                }
                                let _ = self.queue
                                            .push(memcache::set(str::from_utf8(&*self.values[0])
                                                                    .unwrap(),
                                                                str::from_utf8(&*self.values[1])
                                                                    .unwrap(),
                                                                None,
                                                                None)
                                                      .into_bytes());
                            }
                            Protocol::Redis => {
                                let _ = self.queue.push(redis::flushall().into_bytes());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn run(&mut self) {
        self.generate_values(true);
        loop {
            if self.rate != 0 {
                self.ratelimit.block(1);
            }
            self.generate_values(false);

            let query = match self.protocol {
                Protocol::Echo => {
                    match &*self.command {
                        "echo" => echo::echo(&*self.values[0]),
                        _ => {
                            panic!("unknown command: {} for protocol: {:?}",
                                   self.command,
                                   self.protocol);
                        }
                    }
                }
                Protocol::Memcache => {
                    match &*self.command {
                        "set" => {
                            memcache::set(str::from_utf8(&*self.values[0]).unwrap(),
                                          str::from_utf8(&*self.values[1]).unwrap(),
                                          None,
                                          None)
                                .into_bytes()
                        }
                        "get" => {
                            memcache::get(str::from_utf8(&*self.values[0]).unwrap()).into_bytes()
                        }
                        "gets" => {
                            memcache::gets(str::from_utf8(&*self.values[0]).unwrap()).into_bytes()
                        }
                        "add" => {
                            memcache::add(str::from_utf8(&*self.values[0]).unwrap(),
                                          str::from_utf8(&*self.values[1]).unwrap(),
                                          None,
                                          None)
                                .into_bytes()
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
                        "ping" => ping::ping().into_bytes(),
                        _ => {
                            panic!("unknown command: {} for protocol: {:?}",
                                   self.command,
                                   self.protocol);
                        }
                    }
                }
                Protocol::Redis => {
                    match &*self.command {
                        "set" => {
                            redis::set(str::from_utf8(&*self.values[0]).unwrap(),
                                       str::from_utf8(&*self.values[1]).unwrap())
                                .into_bytes()
                        }
                        "get" => redis::get(str::from_utf8(&*self.values[0]).unwrap()).into_bytes(),
                        "hset" => {
                            redis::hset(str::from_utf8(&*self.values[0]).unwrap(),
                                        str::from_utf8(&*self.values[1]).unwrap(),
                                        str::from_utf8(&*self.values[2]).unwrap())
                                .into_bytes()
                        }
                        "hget" => {
                            redis::hget(str::from_utf8(&*self.values[0]).unwrap(),
                                        str::from_utf8(&*self.values[1]).unwrap())
                                .into_bytes()
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
                        "ping" => thrift::ping(),
                        _ => {
                            let mut thrift = thrift::ThriftRequest::default();
                            thrift.method = &self.command;
                            for p in &self.parameters {
                                match p.value {
                                    Value::Stop => {
                                        thrift.payload.push(ThriftType::Stop)
                                    }
                                    Value::Void => {
                                        thrift.payload.push(ThriftType::Void)
                                    }
                                    Value::Bool(v) => {
                                        thrift.payload.push(ThriftType::Bool(p.id, v))
                                    }
                                    Value::Byte(v) => {
                                        thrift.payload.push(ThriftType::Byte(p.id, v))
                                    }
                                    Value::Int16(v) => {
                                        thrift.payload.push(ThriftType::Int16(p.id, v))
                                    }
                                    Value::Int32(v) => {
                                        thrift.payload.push(ThriftType::Int32(p.id, v))
                                    }
                                    Value::Int64(v) => {
                                        thrift.payload.push(ThriftType::Int64(p.id, v))
                                    }
                                    Value::String(ref v) => {
                                        thrift.payload.push(ThriftType::String(p.id, v))
                                    }
                                    Value::Struct => {
                                        thrift.payload.push(ThriftType::Struct(p.id.unwrap()))
                                    }
                                    Value::List(ref ttype, length) => {
                                        thrift.payload.push(ThriftType::List(p.id.unwrap(), ttype, length))
                                    }
                                    _ => {}
                                }
                            }
                            thrift::generic(thrift)
                        }
                    }
                }
                _ => {
                    panic!("unsupported protocol");
                }
            };
            let _ = self.queue.push(query.clone());
        }
    }
}
