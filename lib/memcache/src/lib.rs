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

#![cfg_attr(feature = "unstable", feature(test))]


#[macro_use]
extern crate log;
extern crate rpcperf_cfgtypes as cfgtypes;
extern crate toml;
extern crate getopts;

mod gen;
mod parse;

use cfgtypes::*;
use getopts::Matches;
use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;
use toml::Value;

type Param = Parameter<CacheData>;

#[derive(Clone, Debug)]
enum MemcacheCommand {
    Get(Param),
    Gets(Param),
    Add(Param, Param),
    Set(Param, Param),
}

struct MemcacheParserFactory {
    flush: bool,
}

struct MemcacheParser;

#[derive(Clone, Debug)]
struct CacheData {
    size: usize,
    string: String,
}

impl Ptype for CacheData {
    fn regen(&mut self) {
        self.string = tools::random_string(self.size);
    }

    fn parse(seed: usize, size: usize, _: &BTreeMap<String, Value>) -> CResult<Self> {
        Ok(CacheData {
            size: size,
            string: tools::seeded_string(size, seed),
        })
    }
}

impl ProtocolParseFactory for MemcacheParserFactory {
    fn new(&self) -> Box<ProtocolParse> {
        Box::new(MemcacheParser)
    }

    fn prepare(&self) -> CResult<Vec<Vec<u8>>> {
        Ok(if self.flush {
            vec![gen::flush_all().into_bytes()]
        } else {
            Vec::new()
        })
    }

    fn name(&self) -> &str {
        "memcache"
    }
}

impl ProtocolParse for MemcacheParser {
    fn parse(&self, bytes: &[u8]) -> ParsedResponse {
        let s = str::from_utf8(bytes).unwrap();
        parse::parse_response(s)
    }
}

impl ProtocolGen for MemcacheCommand {
    fn generate_message(&mut self) -> Vec<u8> {
        match *self {
            MemcacheCommand::Set(ref mut key, ref mut val) => {
                key.regen();
                val.regen();
                gen::set(key.value.string.as_str(),
                         val.value.string.as_str(),
                         None,
                         None)
                    .into_bytes()
            }
            MemcacheCommand::Get(ref mut key) => {
                key.regen();
                gen::get(key.value.string.as_str()).into_bytes()
            }
            MemcacheCommand::Gets(ref mut key) => {
                key.regen();
                gen::gets(key.value.string.as_str()).into_bytes()
            }
            MemcacheCommand::Add(ref mut key, ref mut val) => {
                key.regen();
                val.regen();
                gen::add(key.value.string.as_str(),
                         val.value.string.as_str(),
                         None,
                         None)
                    .into_bytes()
            }
        }
    }

    fn method(&self) -> &str {
        match *self {
            MemcacheCommand::Get(_) => "get",
            MemcacheCommand::Gets(_) => "gets",
            MemcacheCommand::Set(_, _) => "set",
            MemcacheCommand::Add(_, _) => "add",
        }
    }
}

/// Load the memcache benchmark configuration from the config toml and command line arguments
pub fn load_config(table: &BTreeMap<String, Value>, matches: &Matches) -> CResult<ProtocolConfig> {

    let mut ws = Vec::new();

    if let Some(&Value::Array(ref workloads)) = table.get("workload") {
        for (i, workload) in workloads.iter().enumerate() {
            if let Value::Table(ref workload) = *workload {
                let w = try!(extract_workload(i, workload));
                ws.push(w);
            } else {
                return Err("malformed config: workload must be a struct".to_owned());
            }
        }

        let protocol = Arc::new(MemcacheParserFactory { flush: matches.opt_present("flush") });

        Ok(ProtocolConfig {
            protocol: protocol,
            workloads: ws,
        })
    } else {
        Err("memcache: no workloads specified".to_owned())
    }
}

fn extract_workload(i: usize, workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {

    let rate = workload.get("rate")
                       .and_then(|k| k.as_integer())
                       .unwrap_or(0);

    let method = workload.get("method")
                         .and_then(|k| k.as_str())
                         .unwrap_or("get")
                         .to_owned();

    let name = workload.get("name")
                       .and_then(|k| k.as_str())
                       .unwrap_or(method.as_str())
                       .to_owned();

    if let Some(&Value::Array(ref params)) = workload.get("parameter") {
        let mut ps = Vec::new();
        for param in params {
            match *param {
                Value::Table(ref parameter) => {
                    let p = try!(extract_parameter(i, parameter));
                    ps.push(p);
                }
                _ => {
                    return Err("malformed config: a parameter must be a struct".to_owned());
                }
            }
        }

        let cmd = match method.as_str() {
            "get" if ps.len() == 1 => MemcacheCommand::Get(ps[0].clone()),
            "gets" if ps.len() == 1 => MemcacheCommand::Gets(ps[0].clone()),
            "set" if ps.len() == 2 => MemcacheCommand::Set(ps[0].clone(), ps[1].clone()),
            "add" if ps.len() == 2 => MemcacheCommand::Add(ps[0].clone(), ps[1].clone()),
            "get" | "gets" | "set" | "add" => {
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
