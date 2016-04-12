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

extern crate log;
extern crate toml;
extern crate rpcperf_request as request;

use getopts::Matches;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use toml::Parser;
use toml::Value;
use toml::Value::{Array, Table};

use request::workload::{Parameter, Style, Type};

type CResult<T> = Result<T, String>;

#[derive(Clone)]
pub struct BenchmarkWorkload {
    pub rate: usize,
    pub method: String,
    pub parameters: Vec<Parameter>,
}

impl Default for BenchmarkWorkload {
    fn default() -> BenchmarkWorkload {
        BenchmarkWorkload {
            rate: 0,
            method: "get".to_owned(),
            parameters: Vec::<Parameter>::new(),
        }
    }
}

pub struct BenchmarkConfig {
    pub connections: usize,
    pub threads: usize,
    pub duration: usize,
    pub windows: usize,
    pub protocol: String,
    pub tcp_nodelay: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub workloads: Vec<BenchmarkWorkload>,
}

impl Default for BenchmarkConfig {
    fn default() -> BenchmarkConfig {
        BenchmarkConfig {
            connections: 1,
            threads: 1,
            duration: 60,
            windows: 5,
            protocol: "memcache".to_owned(),
            tcp_nodelay: false,
            ipv4: true,
            ipv6: true,
            workloads: Vec::new(),
        }
    }
}

/// Helper for extracting non-string values from the `Matches`
fn parse_opt<F>(name: &str, matches: &Matches) -> Result<Option<F>, String>
    where F: FromStr,
          F::Err: Display
{
    if let Some(v) = matches.opt_str(name) {
        match v.parse() {
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(format!("Bad parameter: {}. Cause: {}", name, e)),
        }
    } else {
        Ok(None)
    }
}

pub fn load_config(matches: &Matches) -> Result<BenchmarkConfig, String> {

    // load the config
    if let Some(toml) = matches.opt_str("config") {
        let cfg_txt = match File::open(&toml) {
            Ok(mut f) => {
                let mut cfg_txt = String::new();
                f.read_to_string(&mut cfg_txt).unwrap();
                cfg_txt
            }
            Err(e) => return Err(format!("Error opening config: {}", e)),
        };

        let mut p = Parser::new(&cfg_txt);

        match p.parse() {
            Some(table) => {
                debug!("toml parsed successfully. creating config");
                load_config_table(table, matches)
            }
            None => {
                for err in &p.errors {
                    let (loline, locol) = p.to_linecol(err.lo);
                    let (hiline, hicol) = p.to_linecol(err.hi);
                    println!("{}:{}:{}-{}:{} error: {}",
                             toml,
                             loline,
                             locol,
                             hiline,
                             hicol,
                             err.desc);
                }
                Err("failed to load config".to_owned())
            }
        }
    } else {
        Err("config file not specified".to_owned())
    }
}

pub fn load_config_table(table: BTreeMap<String, Value>,
                         matches: &Matches)
                         -> Result<BenchmarkConfig, String> {
    let mut config: BenchmarkConfig = Default::default();

    if let Some(&Table(ref general)) = table.get("general") {
        if let Some(connections) = general.get("connections")
                                          .and_then(|k| k.as_integer()) {
            config.connections = connections as usize;
        };
        if let Some(threads) = general.get("threads").and_then(|k| k.as_integer()) {
            config.threads = threads as usize;
        }
        if let Some(duration) = general.get("duration")
                                       .and_then(|k| k.as_integer()) {
            config.duration = duration as usize;
        }
        if let Some(windows) = general.get("windows").and_then(|k| k.as_integer()) {
            config.windows = windows as usize;
        }
        if let Some(protocol) = general.get("protocol").and_then(|k| k.as_str()) {
            config.protocol = protocol.to_owned();
        }
        if let Some(tcp_nodelay) = general.get("tcp-nodelay")
                                          .and_then(|k| k.as_bool()) {
            config.tcp_nodelay = tcp_nodelay;
        }
        if let Some(ipv4) = general.get("ipv4").and_then(|k| k.as_bool()) {
            config.ipv4 = ipv4;
        }
        if let Some(ipv6) = general.get("ipv6").and_then(|k| k.as_bool()) {
            config.ipv6 = ipv6;
        }
    }

    // get any overrides from the command line
    try!(config_overrides(&mut config, matches));

    // Load workloads
    match table.get("workload") {
        None => return Err("malformed config: no workload sections".to_owned()),
        Some(&Array(ref workloads)) => {
            for (i, workload) in workloads.iter().enumerate() {
                if let Table(ref workload) = *workload {
                    let w = try!(extract_workload(i, workload));
                    config.workloads.push(w);
                } else {
                    return Err("malformed config: workload must be a struct".to_owned());
                }
            }
        }
        Some(_) => return Err("malformed config: workloads must be an array".to_owned()),
    }

    // double check that we have at least one workload
    if config.workloads.is_empty() {
        Err("malformed config: no worloads specified".to_owned())
    } else {
        Ok(config)
    }
}

/// Override parameters using command line arguments
fn config_overrides(config: &mut BenchmarkConfig, matches: &Matches) -> Result<(), String> {
    // override config with commandline options
    if let Some(protocol) = matches.opt_str("protocol") {
        config.protocol = protocol;
    }

    if let Some(threads) = try!(parse_opt("threads", matches)) {
        config.threads = threads;
    }

    if let Some(connections) = try!(parse_opt("connections", matches)) {
        config.connections = connections;
    }

    if let Some(windows) = try!(parse_opt("windows", matches)) {
        config.windows = windows;
    }

    if let Some(duration) = try!(parse_opt("duration", matches)) {
        config.duration = duration;
    }

    if matches.opt_present("tcp-nodelay") {
        config.tcp_nodelay = true;
    }

    Ok(())
}

fn extract_workload(i: usize, workload: &BTreeMap<String, Value>) -> CResult<BenchmarkWorkload> {
    let mut w: BenchmarkWorkload = Default::default();
    if let Some(method) = workload.get("method").and_then(|k| k.as_str()) {
        w.method = method.to_owned();
    }
    if let Some(rate) = workload.get("rate").and_then(|k| k.as_integer()) {
        w.rate = rate as usize;
    }

    match workload.get("parameter") {
        Some(&Array(ref params)) => {
            for param in params {
                match *param {
                    Table(ref parameter) => {
                        let p = try!(extract_parameter(i, parameter));
                        w.parameters.push(p);
                    }
                    _ => {
                        return Err("malformed config: a parameter must be a struct".to_owned());
                    }
                }
            }
        }
        Some(_) => return Err("malformed config: 'parameter' must be an array".to_owned()),
        None => {}
    }
    Ok(w)
}

fn extract_parameter(i: usize, parameter: &BTreeMap<String, Value>) -> CResult<Parameter> {

    let mut p = Parameter::default();
    p.id = match parameter.get("id")
                          .and_then(|k| k.as_integer()) {
        Some(s) => Some(s as i16),
        None => None,
    };

    p.ptype = match parameter.get("type")
                             .and_then(|k| k.as_str()) {
        Some("stop") => Type::Stop,
        Some("void") => Type::Void,
        Some("bool") => Type::Bool,
        Some("byte") => Type::Byte,
        Some("double") => Type::Double,
        Some("i16") => Type::Int16,
        Some("i32") => Type::Int32,
        Some("i64") => Type::Int64,
        Some("string") => Type::String,
        Some("struct") => Type::Struct,
        Some("map") => Type::Map,
        Some("set") => Type::Set,
        Some("list") => {
            Type::List(parameter.get("contains")
                                .and_then(|k| k.as_str())
                                .unwrap()
                                .to_owned())
        }
        Some(unknown) => return Err(format!("unknown parameter type: {}", unknown)),
        None => Type::None,
    };

    p.style = match parameter.get("style")
                             .and_then(|k| k.as_str()) {
        Some("random") => Style::Random,
        Some("static") => Style::Static,
        None => Style::Static,
        Some(other) => return Err(format!("bad parameter style: {}", other)),
    };

    p.seed = match parameter.get("seed")
                            .and_then(|k| k.as_integer()) {
        Some(s) => s as usize,
        None => i,
    };
    p.size = match parameter.get("size")
                            .and_then(|k| k.as_integer()) {
        Some(s) => s as usize,
        None => 1,
    };
    p.regenerate = match parameter.get("regenerate")
                                  .and_then(|k| k.as_bool()) {
        Some(s) => s,
        None => false,
    };

    Ok(p)
}

#[test]
fn test_load_config() {
    let table = {
        let config_str = include_str!("../configs/thrift_calc.toml");
        let mut p = Parser::new(config_str);
        p.parse().unwrap()
    };

    let matches = {
        let opts = super::opts();
        let args: Vec<String> = Vec::new();
        opts.parse(&args).unwrap()
    };

    let config = load_config_table(table, &matches).unwrap();

    assert_eq!(config.protocol, "thrift");
    assert_eq!(config.workloads.len(), 3);

    let w0 = &config.workloads[0];
    // Check the first workload
    assert_eq!(w0.method, "ping");
    assert_eq!(w0.rate, 1);
    assert_eq!(w0.parameters.len(), 0);

    let w2 = &config.workloads[2];
    // check the third workload
    assert_eq!(w2.method, "calculate");
    assert_eq!(w2.rate, 1);
    assert_eq!(w2.parameters.len(), 6);

    // Check that the first parameter of the third workload was parsed correctly
    assert_eq!(w2.parameters[0].id, Some(1));
    assert_eq!(w2.parameters[0].ptype, Type::Int32);
    assert_eq!(w2.parameters[0].style, Style::Static);
    assert_eq!(w2.parameters[0].regenerate, false);

    // Check that the last parameter was also parsed correctly
    assert_eq!(w2.parameters[5].id, None);
    assert_eq!(w2.parameters[5].ptype, Type::Stop);
    assert_eq!(w2.parameters[5].style, Style::Static);
    assert_eq!(w2.parameters[5].regenerate, false);
}
