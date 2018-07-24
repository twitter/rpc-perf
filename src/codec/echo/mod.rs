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

use cfgtypes::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use toml::Value;

pub struct EchoParser;

struct EchoGen {
    value: Parameter<EchoData>,
}

struct EchoData {
    size: usize,
    bytes: Vec<u8>,
}

impl Default for EchoData {
    fn default() -> EchoData {
        EchoData {
            size: 1,
            bytes: vec![0],
        }
    }
}

impl Ptype for EchoData {
    fn regen(&mut self) {
        self.bytes = tools::random_bytes(self.size);
    }

    fn parse(seed: usize, size: usize, _: &BTreeMap<String, Value>) -> CResult<Self> {
        let bts = (seed..(seed + size)).map(|i| i as u8).collect();
        Ok(EchoData {
            size: size,
            bytes: bts,
        })
    }
}

impl ProtocolGen for EchoGen {
    fn generate_message(&mut self) -> Vec<u8> {
        self.value.regen();
        gen::echo(&self.value.value.bytes)
    }

    fn method(&self) -> &str {
        "echo"
    }
}

impl ProtocolParse for EchoParser {
    fn parse(&self, bytes: &[u8]) -> ParsedResponse {
        parse::parse_response(bytes)
    }
}

impl ProtocolParseFactory for EchoParser {
    fn new(&self) -> Box<ProtocolParse> {
        Box::new(EchoParser)
    }

    fn name(&self) -> &str {
        "echo"
    }
}

/// Load the echo benchmark configuration from the config toml
pub fn load_config(table: &BTreeMap<String, Value>) -> CResult<ProtocolConfig> {
    let mut ws = Vec::new();

    if let Some(&Value::Array(ref workloads)) = table.get("workload") {
        for workload in workloads {
            if let Value::Table(ref workload) = *workload {
                let w = try!(extract_workload(workload));
                ws.push(w);
            } else {
                return Err("malformed config: workload must be a struct".to_owned());
            }
        }

        Ok(ProtocolConfig {
            protocol: Arc::new(EchoParser),
            workloads: ws,
        })
    } else {
        Err("memcache: no workloads specified".to_owned())
    }
}

fn extract_workload(workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {
    let rate = workload
        .get("rate")
        .and_then(|k| k.as_integer())
        .unwrap_or(0);

    let name = workload
        .get("name")
        .and_then(|k| k.as_str())
        .unwrap_or("echo")
        .to_owned();

    if let Some(&Value::Array(ref params)) = workload.get("parameter") {
        let param = match params.len() {
            0 => Parameter::default(),
            1 => {
                if let Value::Table(ref params) = params[0] {
                    try!(extract_parameter(0, params))
                } else {
                    return Err("malformed config: 'parameter' must be a table".to_owned());
                }
            }
            other => {
                return Err(format!(
                    "malformed config: too many parameters for echo: {}",
                    other
                ));
            }
        };

        let gen = Box::new(EchoGen { value: param });

        Ok(BenchmarkWorkload::new(name, rate as usize, gen))
    } else {
        Err("malformed config: 'parameter' must be an array".to_owned())
    }
}
