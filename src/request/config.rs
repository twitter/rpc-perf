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

use super::BenchmarkConfig;
use codec::{echo, memcache, ping, redis_inline, redis_resp, thrift};
use getopts::Matches;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use toml;
use toml::Value::{self, Table};

/// Helper for extracting non-string values from the `Matches`
fn parse_opt<F>(name: &str, matches: &Matches) -> Result<Option<F>, String>
where
    F: FromStr,
    F::Err: Display,
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
    if let Some(toml) = matches.opt_str("config") {
        let cfg_txt = match File::open(&toml) {
            Ok(mut f) => {
                let mut cfg_txt = String::new();
                f.read_to_string(&mut cfg_txt).unwrap();
                cfg_txt
            }
            Err(e) => return Err(format!("Error opening config: {}", e)),
        };

        match toml::from_str(&cfg_txt) {
            Ok(t) => load_config_table(&t, matches),
            Err(e) => {
                if let Some((line, col)) = e.line_col() {
                    error!("Invalid config file: {}", toml);
                    error!("caused by: {}", e);
                    error!("located at: line: {} column: {}", line + 1, col + 1);
                    Err("Invalid config file".to_owned())
                } else {
                    Err("Unknown error in config".to_owned())
                }
            }
        }
    } else {
        Err("config file not specified".to_owned())
    }
}

fn load_config_table(
    table: &BTreeMap<String, Value>,
    matches: &Matches,
) -> Result<BenchmarkConfig, String> {
    let protocol: String = matches
        .opt_str("protocol")
        .or_else(|| {
            table
                .get("general")
                .and_then(|k| k.as_table())
                .and_then(|k| k.get("protocol"))
                .and_then(|k| k.as_str())
                .map(|k| k.to_owned())
        })
        .unwrap_or_else(|| "memcache".to_owned());

    // Pick a protocol
    let proto = match protocol.as_str() {
        "memcache" => try!(memcache::load_config(table, matches)),
        "echo" => try!(echo::load_config(table)),
        "redis" | "redis-resp" | "redis_resp" => try!(redis_resp::load_config(table, matches)),
        "redis-inline" | "redis_inline" => try!(redis_inline::load_config(table, matches)),
        "ping" => try!(ping::load_config(table)),
        "thrift" => try!(thrift::load_config(table)),
        _ => return Err(format!("Protocol {} not known", protocol)),
    };

    if proto.workloads.is_empty() {
        return Err("no workloads specified".to_owned());
    }

    let mut config = BenchmarkConfig::new(proto);

    if let Some(&Table(ref general)) = table.get("general") {
        if let Some(connections) = general.get("connections").and_then(|k| k.as_integer()) {
            config.connections = connections as usize;
        }
        if let Some(threads) = general.get("threads").and_then(|k| k.as_integer()) {
            config.threads = threads as usize;
        }
        if let Some(duration) = general.get("duration").and_then(|k| k.as_integer()) {
            config.duration = duration as usize;
        }
        if let Some(windows) = general.get("windows").and_then(|k| k.as_integer()) {
            config.windows = windows as usize;
        }
        if let Some(tcp_nodelay) = general.get("tcp-nodelay").and_then(|k| k.as_bool()) {
            config.tcp_nodelay = tcp_nodelay;
        }
        if let Some(ipv4) = general.get("ipv4").and_then(|k| k.as_bool()) {
            config.ipv4 = ipv4;
        }
        if let Some(ipv6) = general.get("ipv6").and_then(|k| k.as_bool()) {
            config.ipv6 = ipv6;
        }
        if let Some(v) = general.get("request-timeout").and_then(|k| k.as_integer()) {
            config.set_request_timeout(Some(v as u64));
        }
        if let Some(v) = general.get("connect-timeout").and_then(|k| k.as_integer()) {
            config.set_connect_timeout(Some(v as u64));
        }
        if let Some(v) = general.get("rx-buffer-size").and_then(|k| k.as_integer()) {
            config.set_rx_buffer_size(v as usize);
        }
        if let Some(v) = general.get("tx-buffer-size").and_then(|k| k.as_integer()) {
            config.set_tx_buffer_size(v as usize);
        }
        config.protocol_name = protocol.clone();
    }

    // get any overrides from the command line
    try!(config_overrides(&mut config, matches));

    Ok(config)
}

/// Override parameters using command line arguments
fn config_overrides(config: &mut BenchmarkConfig, matches: &Matches) -> Result<(), String> {
    if let Some(threads) = try!(parse_opt("threads", matches)) {
        config.set_threads(threads);
    }

    if let Some(connections) = try!(parse_opt("connections", matches)) {
        config.set_poolsize(connections);
    }

    if let Some(windows) = try!(parse_opt("windows", matches)) {
        config.set_windows(windows);
    }

    if let Some(duration) = try!(parse_opt("duration", matches)) {
        config.set_duration(duration);
    }

    if let Some(t) = try!(parse_opt("request-timeout", matches)) {
        config.set_request_timeout(Some(t));
    }

    if let Some(t) = try!(parse_opt("connect-timeout", matches)) {
        config.set_connect_timeout(Some(t));
    }

    if matches.opt_present("tcp-nodelay") {
        config.set_tcp_nodelay(true);
    }

    Ok(())
}
