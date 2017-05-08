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
#[cfg(feature = "unstable")]
extern crate test;

#[allow(unknown_lints, useless_attribute)]
#[cfg_attr(feature = "cargo-clippy", deny(result_unwrap_used))]
#[macro_use]
extern crate log;

extern crate bytes;
extern crate byteorder;
extern crate crc;
extern crate getopts;
extern crate log_panics;
extern crate mio;
extern crate mpmc;
extern crate pad;
extern crate time;
extern crate rand;
extern crate ratelimit;
extern crate slab;
extern crate tic;
extern crate toml;

#[macro_use]
mod common;
mod cfgtypes;
mod client;
mod connection;
mod logger;
mod net;
mod options;
mod stats;
mod codec;
mod request;

use client::Client;
use common::*;
use net::InternetProtocol;
use request::{config, workload};
use std::{env, thread};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];
    let opts = options::opts();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("ERROR {}", f);
            options::print_usage(program, &opts);
            process::exit(1);
        }
    };

    if matches.opt_present("help") {
        options::print_usage(program, &opts);
        return;
    }

    if matches.opt_present("version") {
        println!("rpc-perf {}", VERSION);
        return;
    }

    // initialize logging
    logger::set_log_level(matches.opt_count("verbose"));
    log_panics::init();

    info!("rpc-perf {} initializing...", VERSION);
    if cfg!(feature = "asm") {
        info!("feature: asm: enabled");
    } else {
        info!("feature: asm: disabled");
    }

    let servers: Vec<String> = matches.opt_strs("server");

    if servers.is_empty() {
        error!("require server parameter");
        options::print_usage(program, &opts);
        return;
    }

    let listen = matches.opt_str("listen");
    let waterfall = matches.opt_str("waterfall");
    let trace = matches.opt_str("trace");

    // Load workload configuration
    let config = match config::load_config(&matches) {
        Ok(c) => c,
        Err(e) => {
            halt!("{}", e);
        }
    };

    let internet_protocol = match net::choose_layer_3(matches.opt_present("ipv4"),
                                                      matches.opt_present("ipv6")) {
        Ok(i) => i,
        Err(e) => {
            halt!("{}", e);
        }
    };

    print_config(&config, &servers, internet_protocol);

    let stats_receiver = stats::stats_receiver_init(&config, listen, waterfall, trace);

    let mut send_queues = Vec::new();

    let mut client_config = Client::configure();

    client_config
        .set_pool_size(config.poolsize())
        .stats(stats_receiver.get_sender().clone())
        .set_clocksource(stats_receiver.get_clocksource().clone())
        .set_protocol(config.protocol_config.protocol.clone())
        .set_request_timeout(config.request_timeout())
        .set_connect_timeout(config.connect_timeout())
        .set_internet_protocol(internet_protocol);

    for server in servers {
        client_config.add_server(server);
    }

    info!("-----");
    info!("Connecting...");
    for i in 0..config.threads() {
        info!("Client: {}", i);

        let mut client = client_config.clone().build();
        send_queues.push(client.tx());

        let _ = thread::Builder::new()
            .name(format!("client{}", i).to_string())
            .spawn(move || { client.run(); });
    }

    info!("-----");
    info!("Workload:");

    let windows = config.windows();

    workload::launch_workloads(config.protocol_config.workloads,
                               &send_queues,
                               &stats_receiver.get_sender(),
                               &stats_receiver.get_clocksource());

    stats::run(stats_receiver, windows, matches.opt_present("service"));
}

fn print_config(config: &request::BenchmarkConfig,
                servers: &[String],
                internet_protocol: InternetProtocol) {
    info!("-----");
    info!("Config:");
    for server in servers {
        info!("Config: Server: {} Protocol: {}",
              server,
              config.protocol_config.protocol.name());
    }
    info!("Config: IP: {:?} TCP_NODELAY: {}",
          internet_protocol,
          config.tcp_nodelay());
    info!("Config: Threads: {} Poolsize: {}",
          config.threads(),
          config.poolsize());
    info!("Config: Windows: {} Duration: {}",
          config.windows(),
          config.duration());
    match config.request_timeout() {
        Some(v) => {
            info!("Config: Request Timeout: {} ms", v);
        }
        None => {
            info!("Config: Request Timeout: None");
        }
    }
    match config.connect_timeout() {
        Some(v) => {
            info!("Config: Connect Timeout: {} ms", v);
        }
        None => {
            info!("Config: Connect Timeout: None");
        }
    }
}
