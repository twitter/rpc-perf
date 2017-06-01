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

mod gen;
mod parse;

use cfgtypes;
use cfgtypes::{BenchmarkWorkload, CResult, Parameter, ParsedResponse, ProtocolConfig, ProtocolGen,
               ProtocolParse, ProtocolParseFactory, Ptype};
use cfgtypes::tools;
use getopts::Matches;
use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;
use toml::Value;

type Param = Parameter<RedisData>;

#[derive(Clone, Debug)]
struct RedisData {
    size: usize,
    string: String,
}

impl Ptype for RedisData {
    fn regen(&mut self) {
        self.string = tools::random_string(self.size);
    }

    fn parse(seed: usize, size: usize, _: &BTreeMap<String, Value>) -> CResult<Self> {
        Ok(RedisData {
               size: size,
               string: tools::seeded_string(size, seed),
           })
    }
}

enum Command {
    Get(Param),
    Hget(Param, Param),
    Set(Param, Param),
    Hset(Param, Param, Param),
    Del(Param),
    Expire(Param, Param),
    Incr(Param),
    Decr(Param),
    Append(Param, Param),
    Prepend(Param, Param),
}

impl Command {
    fn gen(&mut self) -> Vec<u8> {
        match *self {
            Command::Get(ref mut p1) => {
                p1.regen();
                gen::get(p1.value.string.as_str()).into_bytes()
            }
            Command::Hget(ref mut p1, ref mut p2) => {
                p1.regen();
                p2.regen();
                gen::hget(p1.value.string.as_str(), p2.value.string.as_str()).into_bytes()
            }
            Command::Set(ref mut p1, ref mut p2) => {
                p1.regen();
                p2.regen();
                gen::set(p1.value.string.as_str(), p2.value.string.as_str()).into_bytes()
            }
            Command::Hset(ref mut p1, ref mut p2, ref mut p3) => {
                p1.regen();
                p2.regen();
                p3.regen();
                gen::hset(p1.value.string.as_str(),
                          p2.value.string.as_str(),
                          p3.value.string.as_str())
                        .into_bytes()
            }
            Command::Del(ref mut p1) => {
                p1.regen();
                gen::del(p1.value.string.as_str()).into_bytes()
            }
            Command::Expire(ref mut p1, ref mut p2) => {
                p1.regen();
                gen::expire(p1.value.string.as_str(),
                            p2.value.string.as_str().parse().unwrap())
                        .into_bytes()
            }
            Command::Incr(ref mut p1) => {
                p1.regen();
                gen::incr(p1.value.string.as_str()).into_bytes()
            }
            Command::Decr(ref mut p1) => {
                p1.regen();
                gen::decr(p1.value.string.as_str()).into_bytes()
            }
            Command::Append(ref mut p1, ref mut p2) => {
                p1.regen();
                gen::append(p1.value.string.as_str(), p2.value.string.as_str()).into_bytes()
            }
            Command::Prepend(ref mut p1, ref mut p2) => {
                p1.regen();
                gen::prepend(p1.value.string.as_str(), p2.value.string.as_str()).into_bytes()
            }
        }
    }
}

pub struct RedisParse;

struct RedisParseFactory {
    flush: bool,
    database: u32,
}

impl ProtocolGen for Command {
    fn generate_message(&mut self) -> Vec<u8> {
        self.gen()
    }

    fn method(&self) -> &str {
        match *self {
            Command::Get(_) => "get",
            Command::Set(_, _) => "set",
            Command::Hget(_, _) => "hget",
            Command::Hset(_, _, _) => "hset",
            Command::Del(_) => "del",
            Command::Expire(_, _) => "expire",
            Command::Incr(_) => "incr",
            Command::Decr(_) => "decr",
            Command::Append(_, _) => "append",
            Command::Prepend(_, _) => "prepend",
        }
    }
}

impl ProtocolParseFactory for RedisParseFactory {
    fn new(&self) -> Box<ProtocolParse> {
        Box::new(RedisParse)
    }

    fn prepare(&self) -> CResult<Vec<Vec<u8>>> {
        Ok(if self.flush {
               vec![gen::flushall().into_bytes(),
                    gen::select(&self.database).into_bytes()]
           } else {
               vec![gen::select(&self.database).into_bytes()]
           })
    }

    fn name(&self) -> &str {
        "redis"
    }
}

impl ProtocolParse for RedisParse {
    fn parse(&self, bytes: &[u8]) -> ParsedResponse {
        if let Ok(s) = str::from_utf8(bytes) {
            parse::parse_response(s)
        } else {
            ParsedResponse::Invalid
        }
    }
}

/// Load the redis benchmark configuration from the config toml and command line arguments
pub fn load_config(table: &BTreeMap<String, Value>, matches: &Matches) -> CResult<ProtocolConfig> {

    let mut ws = Vec::new();

    let database = table
        .get("general")
        .and_then(|k| k.as_table())
        .and_then(|k| k.get("database"))
        .and_then(|k| k.as_integer())
        .unwrap_or(0) as u32;

    if let Some(&Value::Array(ref workloads)) = table.get("workload") {
        for workload in workloads.iter() {
            if let Value::Table(ref workload) = *workload {
                ws.push(try!(extract_workload(workload)));
            } else {
                return Err("workload must be table".to_owned());
            }
        }

        let proto = Arc::new(RedisParseFactory {
                                 flush: matches.opt_present("flush"),
                                 database: database,
                             });

        Ok(ProtocolConfig {
               protocol: proto,
               workloads: ws,
           })
    } else {
        Err("no workloads specified".to_owned())
    }
}

fn extract_workload(workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {
    let rate = workload
        .get("rate")
        .and_then(|k| k.as_integer())
        .unwrap_or(0);

    let method = workload
        .get("method")
        .and_then(|k| k.as_str())
        .unwrap_or("get")
        .to_owned();

    let name = workload
        .get("name")
        .and_then(|k| k.as_str())
        .unwrap_or_else(|| method.as_str())
        .to_owned();

    if let Some(&Value::Array(ref params)) = workload.get("parameter") {
        let mut ps = Vec::new();
        for (i, param) in params.iter().enumerate() {
            match *param {
                Value::Table(ref parameter) => {
                    let p = try!(cfgtypes::extract_parameter(i, parameter));
                    ps.push(p);
                }
                _ => {
                    return Err("malformed config: a parameter must be a struct".to_owned());
                }
            }
        }

        let cmd = match method.as_str() {
            "get" if ps.len() == 1 => Command::Get(ps[0].clone()),
            "hget" if ps.len() == 2 => Command::Hget(ps[0].clone(), ps[1].clone()),
            "set" if ps.len() == 2 => Command::Set(ps[0].clone(), ps[1].clone()),
            "hset" if ps.len() == 3 => Command::Hset(ps[0].clone(), ps[1].clone(), ps[2].clone()),
            "del" if ps.len() == 1 => Command::Del(ps[0].clone()),
            "expire" if ps.len() == 2 => Command::Expire(ps[0].clone(), ps[1].clone()),
            "incr" if ps.len() == 1 => Command::Incr(ps[0].clone()),
            "decr" if ps.len() == 1 => Command::Decr(ps[0].clone()),
            "append" if ps.len() == 2 => Command::Append(ps[0].clone(), ps[1].clone()),
            "prepend" if ps.len() == 1 => Command::Prepend(ps[0].clone(), ps[1].clone()),
            "get" | "set" | "hset" | "hget" | "del" | "expire" | "incr" | "decr" | "append" |
            "prepend" => {
                return Err(format!("invalid number of params ({}) for method {}",
                                   ps.len(),
                                   method));
            }
            _ => return Err(format!("invalid command: {}", method)),
        };

        Ok(BenchmarkWorkload::new(name, rate as usize, Box::new(cmd)))
    } else {
        Err("malformed config: 'parameter' must be an array".to_owned())
    }
}
