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


use super::{Parameter, Tvalue};
use super::gen;
use super::parse;
use cfgtypes::{BenchmarkWorkload, CResult, ParsedResponse, ProtocolConfig, ProtocolGen,
               ProtocolParse, ProtocolParseFactory, Style, tools};
use cfgtypes::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct ThriftParse;
struct ThriftParseFactory;

struct ThriftGen {
    method: String,
    parameters: Vec<Parameter>,
}

impl ProtocolGen for ThriftGen {
    fn generate_message(&mut self) -> Vec<u8> {
        if "ping" == self.method.as_str() {
            gen::ping()
        } else {
            gen::generic(&self.method, 0, &mut self.parameters)
        }
    }

    fn method(&self) -> &str {
        &self.method
    }
}

impl ProtocolParseFactory for ThriftParseFactory {
    fn new(&self) -> Box<ProtocolParse> {
        Box::new(ThriftParse)
    }

    fn name(&self) -> &str {
        "thrift"
    }
}

impl ProtocolParse for ThriftParse {
    fn parse(&self, bytes: &[u8]) -> ParsedResponse {
        parse::parse_response(bytes)
    }
}

/// Load the thrift benchmark configuration from the config toml
pub fn load_config(table: &BTreeMap<String, Value>) -> CResult<ProtocolConfig> {
    let mut ws = Vec::new();

    if let Some(&Value::Array(ref workloads)) = table.get("workload") {
        for workload in workloads.iter() {
            if let Value::Table(ref workload) = *workload {
                ws.push(try!(extract_workload(workload)));
            } else {
                return Err("workload must be table".to_owned());
            }
        }

        Ok(ProtocolConfig {
            protocol: Arc::new(ThriftParseFactory),
            workloads: ws,
        })
    } else {
        Err("no workloads specified".to_owned())
    }
}

fn extract_workload(workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {
    let method = match workload.get("method").and_then(|k| k.as_str()) {
        Some(m) => m,
        None => return Err("malformed config: 'method' not specified".to_owned()),
    };

    let rate = workload
        .get("rate")
        .and_then(|k| k.as_integer())
        .unwrap_or(0);

    let name = workload
        .get("name")
        .and_then(|k| k.as_str())
        .unwrap_or(method)
        .to_owned();

    let mut ps = Vec::new();

    match workload.get("parameter") {
        Some(&Value::Array(ref params)) => {
            for (i, param) in params.iter().enumerate() {
                if let Value::Table(ref parameter) = *param {
                    let p = try!(extract_parameter(i, parameter));
                    ps.push(p);
                } else {
                    return Err("malformed config: a parameter must be a struct".to_owned());
                }
            }
        }
        Some(_) => return Err("malformed config: 'parameter' must be an array".to_owned()),
        None => {}
    }

    let cmd = Box::new(ThriftGen {
        method: method.to_owned(),
        parameters: ps,
    });

    Ok(BenchmarkWorkload::new(name, rate as usize, cmd))
}

fn extract_parameter(i: usize, parameter: &BTreeMap<String, Value>) -> CResult<Parameter> {

    let id = parameter.get("id").and_then(|k| k.as_integer()).map(
        |k| k as i16,
    );

    let style = match parameter.get("style").and_then(|k| k.as_str()) {
        Some("random") => Style::Random,
        Some("static") | None => Style::Static,
        Some(other) => return Err(format!("bad parameter style: {}", other)),
    };

    let seed = match parameter.get("seed").and_then(|k| k.as_integer()) {
        Some(s) => s as usize,
        None => i,
    };

    let size = match parameter.get("size").and_then(|k| k.as_integer()) {
        Some(s) => s as usize,
        None => 1,
    };

    let regenerate = match parameter.get("regenerate").and_then(|k| k.as_bool()) {
        Some(s) => s,
        None => false,
    };

    let mut value = match parameter.get("type").and_then(|k| k.as_str()) {
        Some("stop") => Tvalue::Stop,
        Some("void") => Tvalue::Void,
        Some("bool") => Tvalue::Bool(true),
        Some("byte") => Tvalue::Byte(i as u8),
        Some("double") => Tvalue::Double(i as f64),
        Some("i16") => Tvalue::Int16(i as i16),
        Some("i32") => Tvalue::Int32(i as i32),
        Some("i64") => Tvalue::Int64(i as i64),
        Some("string") => Tvalue::String(tools::seeded_string(size, seed)),
        Some("struct") => Tvalue::Struct,
        Some("map") => Tvalue::Map,
        Some("set") => Tvalue::Set,
        Some("list") => {
            Tvalue::List(
                parameter
                    .get("contains")
                    .and_then(|k| k.as_str())
                    .unwrap()
                    .to_owned(),
                size as i32,
            )
        }
        Some(unknown) => return Err(format!("unknown parameter type: {}", unknown)),
        None => return Err("paramter type not specified".to_owned()),
    };

    if style == Style::Random {
        value.regen(size);
    }

    Ok(Parameter {
        id: id,
        seed: seed,
        size: size,
        style: style,
        regenerate: regenerate,
        value: value,
    })
}

#[test]
fn test_load_config() {
    use cfgtypes::Parser;

    let table = {
        let config_str = include_str!("../../../configs/thrift_calc.toml");
        let mut p = Parser::new(config_str);
        p.parse().unwrap()
    };

    let config = load_config(&table).unwrap();

    assert_eq!(config.protocol.name(), "thrift");
    assert_eq!(config.workloads.len(), 3);

    let w0 = &config.workloads[0];
    // Check the first workload
    assert_eq!(w0.name, "ping");
    assert_eq!(w0.gen.method(), "ping");
    assert_eq!(w0.rate, 1);

    let w2 = &config.workloads[2];
    // check the third workload
    assert_eq!(w2.name, "sub");
    assert_eq!(w2.gen.method(), "calculate");
    assert_eq!(w2.rate, 1);
}
