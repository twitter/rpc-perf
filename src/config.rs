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
extern crate workload;

use std::fs::File;
use std::io::Read;
use toml::Parser;
use toml::Value::Table;
use workload::Parameter;

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

pub fn load_config(path: String) -> Result<BenchmarkConfig, &'static str> {
    let mut f = File::open(&path).unwrap();

    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let mut p = Parser::new(&s);

    match p.parse() {
        Some(toml) => {
            debug!("toml parsed successfully. creating config");

            let mut config: BenchmarkConfig = Default::default();
            let table = Table(toml);

            if let Some(general) = table.lookup("general") {
                if let Some(connections) = general.lookup("connections")
                                                  .and_then(|k| k.as_integer()) {
                    config.connections = connections as usize;
                };
                if let Some(threads) = general.lookup("threads").and_then(|k| k.as_integer()) {
                    config.threads = threads as usize;
                }
                if let Some(duration) = general.lookup("duration")
                                               .and_then(|k| k.as_integer()) {
                    config.duration = duration as usize;
                }
                if let Some(windows) = general.lookup("windows").and_then(|k| k.as_integer()) {
                    config.windows = windows as usize;
                }
                if let Some(protocol) = general.lookup("protocol").and_then(|k| k.as_str()) {
                    config.protocol = protocol.to_owned();
                }
                if let Some(tcp_nodelay) = general.lookup("tcp-nodelay")
                                                  .and_then(|k| k.as_bool()) {
                    config.tcp_nodelay = tcp_nodelay;
                }
                if let Some(ipv4) = general.lookup("ipv4").and_then(|k| k.as_bool()) {
                    config.ipv4 = ipv4;
                }
                if let Some(ipv6) = general.lookup("ipv6").and_then(|k| k.as_bool()) {
                    config.ipv6 = ipv6;
                }
            }

            let mut i = 0;
            loop {
                let key = format!("workload.{}", i);
                match table.lookup(&key) {
                    Some(workload) => {
                        debug!("workload: {} defined", i);
                        let mut w: BenchmarkWorkload = Default::default();
                        if let Some(method) = workload.lookup("method").and_then(|k| k.as_str()) {
                            w.method = method.to_owned();
                        }
                        if let Some(rate) = workload.lookup("rate").and_then(|k| k.as_integer()) {
                            w.rate = rate as usize;
                        }
                        let mut j = 0;
                        loop {
                            let param_key = format!("parameter.{}", j);
                            match workload.lookup(&param_key) {
                                Some(parameter) => {
                                    let style = match parameter.lookup("style")
                                                               .and_then(|k| k.as_str()) {
                                        Some(s) => s,
                                        None => "static",
                                    };
                                    let seed = match parameter.lookup("seed")
                                                              .and_then(|k| k.as_integer()) {
                                        Some(s) => s as usize,
                                        None => i,
                                    };
                                    let size = match parameter.lookup("size")
                                                              .and_then(|k| k.as_integer()) {
                                        Some(s) => s as usize,
                                        None => 1,
                                    };
                                    let regenerate = match parameter.lookup("regenerate")
                                                                    .and_then(|k| k.as_bool()) {
                                        Some(s) => s,
                                        None => false,
                                    };
                                    match style {
                                        "random" => {
                                            w.parameters.push(workload::Parameter::Random {
                                                size: size,
                                                regenerate: regenerate,
                                            });
                                        }
                                        "static" => {
                                            w.parameters.push(workload::Parameter::Static {
                                                size: size,
                                                seed: seed,
                                            });
                                        }
                                        _ => {
                                            panic!("bad parameter style: {}", style);
                                        }
                                    }
                                    j += 1;
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                        config.workloads.push(w);
                    }
                    None => {
                        break;
                    }
                }
                i += 1;
            }
            if i < 1 {
                return Err("no workload section");
            }
            return Ok(config);
        }
        None => {
            for err in &p.errors {
                let (loline, locol) = p.to_linecol(err.lo);
                let (hiline, hicol) = p.to_linecol(err.hi);
                println!("{}:{}:{}-{}:{} error: {}",
                         path,
                         loline,
                         locol,
                         hiline,
                         hicol,
                         err.desc);
            }
        }
    }
    Err("failed to load config")
}
