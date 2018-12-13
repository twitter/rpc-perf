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

use cfgtypes::{BenchmarkWorkload, CResult, ParsedResponse, ProtocolConfig, ProtocolGen,
               ProtocolParse, ProtocolParseFactory};
use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;
use toml::Value;

#[derive(Clone)]
pub struct Ping;

impl ProtocolGen for Ping {
    fn generate_message(&mut self) -> Vec<u8> {
        gen::ping().into_bytes()
    }

    fn method(&self) -> &str {
        "ping"
    }

    fn boxed(&self) -> Box<ProtocolGen> {
        Box::new(self.clone())
    }
}

impl ProtocolParseFactory for Ping {
    fn new(&self) -> Box<ProtocolParse> {
        Box::new(Ping)
    }

    fn name(&self) -> &str {
        "ping"
    }
}

impl ProtocolParse for Ping {
    fn parse(&self, bytes: &[u8]) -> ParsedResponse {
        if let Ok(s) = str::from_utf8(bytes) {
            parse::parse_response(s)
        } else {
            ParsedResponse::Invalid
        }
    }
}

/// Load the ping benchmark configuration from the config toml
pub fn load_config(table: &BTreeMap<String, Value>) -> CResult<ProtocolConfig> {
    let mut ws = Vec::new();

    if let Some(&Value::Array(ref workloads)) = table.get("workload") {
        for workload in workloads.iter() {
            if let Value::Table(ref workload) = *workload {
                ws.push(extract_workload(workload)?);
            } else {
                return Err("workload must be a table".to_owned());
            }
        }

        Ok(ProtocolConfig {
            protocol: Arc::new(Ping),
            workloads: ws,
            warmups: Vec::new(),
        })
    } else {
        Err("no workload specified".to_owned())
    }
}

fn extract_workload(workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {
    let rate = workload
        .get("rate")
        .and_then(|k| k.as_integer())
        .unwrap_or(0);

    if let Some(v) = workload.get("method").and_then(|s| s.as_str()) {
        if v != "ping" {
            return Err(format!("invalid method: {}", v));
        }
    }

    let name = workload
        .get("name")
        .and_then(|k| k.as_str())
        .unwrap_or("ping")
        .to_owned();

    Ok(BenchmarkWorkload::new(name, rate as usize, Box::new(Ping)))
}
