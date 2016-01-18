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

use std::fs::File;
use std::io::Read;
use toml::Parser;
use toml::Value::Table;

#[derive(Clone)]
pub struct BenchmarkWorkload {
    pub rate: usize,
    pub method: String,
    pub bytes: usize,
    pub bucket: usize,
    pub hit: bool,
    pub flush: bool,
}

impl Default for BenchmarkWorkload {
    fn default() -> BenchmarkWorkload {
        BenchmarkWorkload {
            rate: 0,
            method: "get".to_string(),
            bytes: 1,
            bucket: 10000,
            hit: false,
            flush: false,
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
            protocol: "memcache".to_string(),
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

            match table.lookup("general") {
                Some(general) => {
                    match general.lookup("connections").and_then(|k| k.as_integer()) {
                        Some(connections) => {
                            config.connections = connections as usize;
                        }
                        None => {}
                    }
                    match general.lookup("threads").and_then(|k| k.as_integer()) {
                        Some(threads) => {
                            config.threads = threads as usize;
                        }
                        None => {}
                    }
                    match general.lookup("duration").and_then(|k| k.as_integer()) {
                        Some(duration) => {
                            config.duration = duration as usize;
                        }
                        None => {}
                    }
                    match general.lookup("windows").and_then(|k| k.as_integer()) {
                        Some(windows) => {
                            config.windows = windows as usize;
                        }
                        None => {}
                    }
                    match general.lookup("protocol").and_then(|k| k.as_str()) {
                        Some(protocol) => {
                            config.protocol = protocol.to_string();
                        }
                        None => {}
                    }
                    match general.lookup("tcp-nodelay").and_then(|k| k.as_bool()) {
                        Some(tcp_nodelay) => {
                            config.tcp_nodelay = tcp_nodelay;
                        }
                        None => {}
                    }
                    match general.lookup("ipv4").and_then(|k| k.as_bool()) {
                        Some(ipv4) => {
                            config.ipv4 = ipv4;
                        }
                        None => {}
                    }
                    match general.lookup("ipv6").and_then(|k| k.as_bool()) {
                        Some(ipv6) => {
                            config.ipv6 = ipv6;
                        }
                        None => {}
                    }
                }
                None => {
                    return Err("config has no general section");
                }
            }

            let mut i = 0;
            loop {
                let key = format!("workload.{}", i);
                match table.lookup(&key) {
                    Some(workload) => {
                        debug!("workload: {} defined", i);
                        let mut w: BenchmarkWorkload = Default::default();
                        match workload.lookup("method").and_then(|k| k.as_str()) {
                            Some(method) => {
                                w.method = method.to_string();
                            }
                            None => {}
                        }
                        match workload.lookup("rate").and_then(|k| k.as_integer()) {
                            Some(rate) => {
                                w.rate = rate as usize;
                            }
                            None => {}
                        }
                        match workload.lookup("bytes").and_then(|k| k.as_integer()) {
                            Some(bytes) => {
                                w.bytes = bytes as usize;
                            }
                            None => {}
                        }
                        match workload.lookup("hit").and_then(|k| k.as_bool()) {
                            Some(hit) => {
                                w.hit = hit;
                            }
                            None => {}
                        }
                        match workload.lookup("flush").and_then(|k| k.as_bool()) {
                            Some(flush) => {
                                w.flush = flush;
                            }
                            None => {}
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
