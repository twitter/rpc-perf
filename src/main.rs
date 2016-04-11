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

#[macro_use]
extern crate log;

extern crate bytes;
extern crate getopts;
extern crate heatmap;
extern crate histogram;
extern crate time;
extern crate mio;
extern crate mpmc;
extern crate regex;
extern crate rpcperf_parser as parser;
extern crate rpcperf_request as request;
extern crate shuteye;
extern crate toml;
extern crate waterfall;

pub mod client;
pub mod config;
pub mod connection;
pub mod logger;
pub mod net;
pub mod state;
pub mod stats;

use getopts::Options;
use log::LogLevelFilter;
use mpmc::Queue as BoundedQueue;
use std::env;
use std::net::ToSocketAddrs;
use std::thread;
use std::sync::mpsc;
use std::process;


use client::Client;
use config::BenchmarkConfig;
use connection::Connection;
use logger::SimpleLogger;
use net::InternetProtocol;
use stats::Stat;
use request::workload::{Protocol, Workload};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const BUCKET_SIZE: usize = 10_000;

struct ClientConfig {
    servers: Vec<String>,
    connections: usize,
    stats_tx: mpsc::Sender<Stat>,
    client_protocol: Protocol,
    internet_protocol: InternetProtocol,
    work_rx: BoundedQueue<Vec<u8>>,
    tcp_nodelay: bool,
    mio_config: mio::EventLoopConfig,
}

fn start(config: ClientConfig) {
    let mut event_loop = mio::EventLoop::configured(config.mio_config.clone()).unwrap();
    let mut client = Client::new(config.work_rx.clone());

    let mut failures = 0;
    let mut connects = 0;

    for server in &config.servers {
        let address = &server.to_socket_addrs().unwrap().next().unwrap();
        for _ in 0..config.connections {
            match net::to_mio_tcp_stream(address, config.internet_protocol) {
                Ok(stream) => {
                    match client.connections.insert_with(|token| {
                        Connection::new(stream,
                                        token,
                                        config.stats_tx.clone(),
                                        config.client_protocol,
                                        config.tcp_nodelay)
                    }) {
                        Some(token) => {
                            event_loop.register(&client.connections[token].socket,
                                                token,
                                                mio::EventSet::writable(),
                                                mio::PollOpt::edge() | mio::PollOpt::oneshot())
                                      .unwrap();
                            connects += 1;
                        }
                        _ => debug!("too many established connections"),
                    }
                }
                Err(e) => {
                    failures += 1;
                    debug!("connect error: {}", e);
                }
            }
        }
    }
    info!("Connections: {} Failures: {}", connects, failures);
    if failures == config.connections {
        error!("All connections have failed");
        process::exit(1);
    } else {
        event_loop.run(&mut client).unwrap();
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn opts() -> Options {
    let mut opts = Options::new();

    opts.optmulti("s", "server", "server address", "HOST:PORT");
    opts.optopt("t", "threads", "number of threads", "INTEGER");
    opts.optopt("c", "connections", "connections per thread", "INTEGER");
    opts.optopt("d", "duration", "number of seconds per window", "INTEGER");
    opts.optopt("w", "windows", "number of windows in test", "INTEGER");
    opts.optopt("p", "protocol", "client protocol", "STRING");
    opts.optopt("", "config", "TOML config file", "FILE");
    opts.optopt("", "trace", "write histogram data to file", "FILE");
    opts.optopt("", "waterfall", "output waterfall PNG", "FILE");
    opts.optflag("", "tcp-nodelay", "enable tcp nodelay");
    opts.optflag("", "flush", "flush cache prior to test");
    opts.optflag("", "ipv4", "force IPv4 only");
    opts.optflag("", "ipv6", "force IPv6 only");
    opts.optflag("", "version", "show version and exit");
    opts.optflagmulti("v", "verbose", "verbosity (stacking)");
    opts.optflag("h", "help", "print this help menu");

    opts
}

fn set_log_level(level: usize) {
    let log_filter;
    match level {
        0 => {
            log_filter = LogLevelFilter::Info;
        }
        1 => {
            log_filter = LogLevelFilter::Debug;
        }
        _ => {
            log_filter = LogLevelFilter::Trace;
        }
    }
    let _ = log::set_logger(|max_log_level| {
        max_log_level.set(log_filter);
        Box::new(SimpleLogger)
    });
}

fn choose_layer_3(ipv4: bool, ipv6: bool) -> Result<InternetProtocol, String> {
    if ipv4 && ipv6 {
        return Err("Use only --ipv4 or --ipv6".to_owned());
    }

    if !ipv4 && !ipv6 {
        return Ok(InternetProtocol::Any);
    } else if ipv4 {
        return Ok(InternetProtocol::IpV4);
    } else if ipv6 {
        return Ok(InternetProtocol::IpV6);
    }

    Err("No InternetProtocols remaining! Bad config/options".to_owned())
}

fn launch_workloads(protocol: String,
                    workloads: Vec<config::BenchmarkWorkload>,
                    work_queue: mpmc::Queue<Vec<u8>>) {

    for (i, w) in workloads.iter().enumerate() {
        info!("Workload {}: Method: {} Rate: {}", i, w.method, w.rate);

        let protocol = Protocol::new(&protocol.clone()).unwrap();

        let mut workload = Workload::new(protocol,
                                         w.method.clone(),
                                         Some(w.rate as u64),
                                         work_queue.clone())
                               .unwrap();

        for p in &w.parameters {
            info!("Parameter: {:?}", p);
            workload.add_param(p.clone());
        }


        thread::spawn(move || {
            loop {
                workload.run();
            }
        });
    }
}

fn load_config(file: Option<String>) -> BenchmarkConfig {
    let mut config: BenchmarkConfig = Default::default();

     // load config from file if specified
    if let Some(toml) = file {
        match config::load_config(&toml) {
            Ok(cfg) => {
                config = cfg;
            }
            Err(msg) => {
                error!("{}", msg);
                panic!();
            }
        }
    }

    config
}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let program = &args[0];

    let opts = opts();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("help") {
        print_usage(&program, opts);
        return;
    }

    if matches.opt_present("version") {
        println!("rpc-perf {}", VERSION);
        return;
    }

    // defaults
    let client_protocol: Protocol;

    set_log_level(matches.opt_count("verbose"));

    info!("rpc-perf {} initializing...", VERSION);

    if matches.opt_count("server") < 1 {
        error!("require server parameter");
        print_usage(&program, opts);
        return;
    };

    let waterfall = matches.opt_str("waterfall");
    let trace = matches.opt_str("trace");

    let mut config = load_config(matches.opt_str("config"));

    // override config with commandline options
    config.override_protocol(matches.opt_str("protocol"));
    config.override_threads(matches.opt_str("threads"));
    config.override_connections(matches.opt_str("connections"));
    config.override_windows(matches.opt_str("windows"));
    config.override_duration(matches.opt_str("duration"));

    let internet_protocol = match choose_layer_3(matches.opt_present("ipv4"),
                                                 matches.opt_present("ipv6")) {
        Ok(i) => i,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    if matches.opt_present("tcp-nodelay") {
        config.tcp_nodelay = true;
    }

    let work_queue = BoundedQueue::<Vec<u8>>::with_capacity(BUCKET_SIZE);

    // these map to workload and conflict with config for simplicity
    if config.workloads.is_empty() {
        error!("configuration contains no workload sections");
        return;
    }

    match Protocol::new(&*config.protocol) {
        Ok(p) => {
            client_protocol = p;
        }
        Err(_) => {
            error!("Bad protocol: {}", &*config.protocol);
            return;
        }
    }

    if matches.opt_present("flush") {
        match client_protocol {
            Protocol::Memcache => {
                let _ = work_queue.push(request::memcache::flush_all().into_bytes());
            }
            Protocol::Redis => {
                let _ = work_queue.push(request::redis::flushall().into_bytes());
            }
            _ => {}
        }
    }

    let evconfig = mio::EventLoopConfig::default();

    info!("-----");
    info!("Config:");
    for server in matches.opt_strs("server") {
        info!("Config: Server: {} Protocol: {}", server, config.protocol);
    }
    info!("Config: IP: {:?} TCP_NODELAY: {}",
          internet_protocol,
          config.tcp_nodelay);
    info!("Config: Threads: {} Connections: {}",
          config.threads,
          config.connections);
    info!("Config: Windows: {} Duration: {}",
          config.windows,
          config.duration);
    info!("-----");
    info!("Workload:");

    launch_workloads(config.protocol, config.workloads, work_queue.clone());

    let (stats_sender, stats_receiver) = mpsc::channel();

    let receiver = stats::Receiver::new(stats_receiver);

    info!("-----");
    info!("Connecting...");
    // spawn client threads
    for i in 0..config.threads {
        info!("Client: {}", i);

        let client_config = ClientConfig {
            servers: matches.opt_strs("server"),
            connections: config.connections,
            stats_tx: stats_sender.clone(),
            client_protocol: client_protocol,
            internet_protocol: internet_protocol,
            work_rx: work_queue.clone(),
            tcp_nodelay: config.tcp_nodelay,
            mio_config: evconfig.clone(),
        };

        thread::spawn(move || {
            start(client_config);
        });
    }

    receiver.run(config.duration, config.windows, trace, waterfall);
}
