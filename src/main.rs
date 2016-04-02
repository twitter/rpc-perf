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
extern crate rpcperf_workload as workload;
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
use heatmap::{Heatmap, HeatmapConfig};
use histogram::{Histogram, HistogramConfig};
use log::LogLevelFilter;
use mpmc::Queue as BoundedQueue;
use std::env;
use std::net::ToSocketAddrs;
use std::thread;
use std::sync::mpsc;
use std::process;
use waterfall::Waterfall;

use client::Client;
use config::BenchmarkConfig;
use connection::Connection;
use logger::SimpleLogger;
use net::InternetProtocol;
use stats::{Stat, Status};
use workload::{Protocol, Workload};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const ONE_SECOND: isize = 1_000_000_000;
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

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

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
    opts.optflagmulti("v", "verbose", "verbosity (stacking)");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("help") {
        print_usage(&program, opts);
        return;
    }

    // defaults
    let log_filter;
    let mut config: BenchmarkConfig = Default::default();
    let client_protocol: Protocol;

    match matches.opt_count("verbose") {
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

    if matches.opt_count("server") < 1 {
        error!("require server parameter");
        print_usage(&program, opts);
        return;
    };

    let trace = matches.opt_str("trace");

    // load config from file if specified
    if let Some(toml) = matches.opt_str("config") {
        config = config::load_config(toml).unwrap();
    }

    // override config with commandline options

    // these map to general section, and can override config
    if let Some(protocol) = matches.opt_str("protocol") {
        config.protocol = protocol;
    }

    if let Some(t) = matches.opt_str("threads") {
        match t.parse() {
            Ok(threads) => {
                if threads > 0 {
                    config.threads = threads;
                } else {
                    error!("Bad parameter: {} Cause: {}",
                           "threads",
                           "not greater than zero");
                    return;
                }
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "threads", e);
                return;
            }
        }
    }

    if let Some(c) = matches.opt_str("connections") {
        match c.parse() {
            Ok(connections) => {
                if connections > 0 {
                    config.connections = connections;
                } else {
                    error!("Bad parameter: {} Cause: {}",
                           "connections",
                           "not greater than zero");
                    return;
                }
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "connections", e);
                return;
            }
        }
    }

    if let Some(w) = matches.opt_str("windows") {
        match w.parse() {
            Ok(windows) => {
                if windows > 0 {
                    config.windows = windows;
                } else {
                    error!("Bad parameter: {} Cause: {}",
                           "windows",
                           "not greater than zero");
                    return;
                }
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "windows", e);
                return;
            }
        }
    }

    if let Some(d) = matches.opt_str("duration") {
        match d.parse() {
            Ok(duration) => {
                if duration > 0 {
                    config.duration = duration;
                } else {
                    error!("Bad parameter: {} Cause: {}",
                           "duration",
                           "not greater than zero");
                    return;
                }
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "duration", e);
                return;
            }
        }
    }

    if matches.opt_present("tcp-nodelay") {
        config.tcp_nodelay = true;
    }

    let workq = BoundedQueue::<Vec<u8>>::with_capacity(BUCKET_SIZE);

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
            panic!("Bad protocol: {}", &*config.protocol);
        }
    }

    if matches.opt_present("flush") {
        match client_protocol {
            Protocol::Memcache => {
                let _ = workq.push(request::memcache::flush_all().into_bytes());
            }
            Protocol::Redis => {
                let _ = workq.push(request::redis::flushall().into_bytes());
            }
            _ => {}
        }
    }

    let mut internet_protocol = InternetProtocol::None;

    if matches.opt_present("ipv4") && matches.opt_present("ipv6") {
        error!("Use only --ipv4 or --ipv6");
        print_usage(&program, opts);
        return;
    }

    if matches.opt_present("ipv4") {
        config.ipv4 = true;
        config.ipv6 = false;
    }
    if matches.opt_present("ipv6") {
        config.ipv4 = false;
        config.ipv6 = true;
    }
    if config.ipv4 && config.ipv6 {
        internet_protocol = InternetProtocol::Any;
    } else if config.ipv4 {
        internet_protocol = InternetProtocol::IpV4;
    } else if config.ipv6 {
        internet_protocol = InternetProtocol::IpV6;
    }
    if internet_protocol == InternetProtocol::None {
        error!("No InternetProtocols remaining! Bad config/options");
        return;
    }

    let evconfig = mio::EventLoopConfig::default();

    info!("rpc-perf {} initializing...", VERSION);
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

    for (i,w) in config.workloads.iter().enumerate() {
        //let w = &config.workloads[i];
        info!("Workload {}: Method: {} Rate: {}", i, w.method, w.rate);

        let protocol = Protocol::new(&config.protocol.clone()).unwrap();

        let mut workload = Workload::new(protocol, w.method.clone(), Some(w.rate as u64), workq.clone())
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

    let (stats_tx, stats_rx) = mpsc::channel();

    info!("-----");
    info!("Connecting...");
    // spawn client threads
    for i in 0..config.threads {
        info!("Client: {}", i);
        let stats_tx = stats_tx.clone();
        let servers = matches.opt_strs("server");
        let connections = config.connections;
        let work_rx = workq.clone();
        let tcp_nodelay = config.tcp_nodelay;
        let internet_protocol = internet_protocol;
        let evconfig = evconfig.clone();

        let client_config = ClientConfig {
            servers: servers,
            connections: connections,
            stats_tx: stats_tx,
            client_protocol: client_protocol,
            internet_protocol: internet_protocol,
            work_rx: work_rx,
            tcp_nodelay: tcp_nodelay,
            mio_config: evconfig,
        };

        thread::spawn(move || {
            start(client_config);
        });
    }

    let mut histogram_config = HistogramConfig::new();
    histogram_config.precision(4).max_value(60 * ONE_SECOND as u64);
    let mut histogram = Histogram::configured(histogram_config).unwrap();

    let mut heatmap_config = HeatmapConfig::new();
    heatmap_config.precision(2).max_value(ONE_SECOND as u64);
    heatmap_config.slice_duration(ONE_SECOND as u64).num_slices((config.duration * config.windows));
    let mut heatmap = Heatmap::configured(heatmap_config).unwrap();

    let mut printed_at = time::precise_time_ns();
    let mut ok = 0_u64;
    let mut hit = 0_u64;
    let mut miss = 0_u64;
    let mut error = 0_u64;
    let mut closed = 0_u64;
    let mut window = 0;
    let mut warmup = true;

    loop {
        match stats_rx.try_recv() {
            Ok(result) => {
                match result.status {
                    Status::Ok => {
                        ok += 1;
                    }
                    Status::Hit => {
                        hit += 1;
                        ok += 1;
                    }
                    Status::Miss => {
                        miss += 1;
                    }
                    Status::Error => {
                        error += 1;
                    }
                    Status::Closed => {
                        closed += 1;
                    }
                }
                let _ = histogram.increment(result.stop - result.start);
                let _ = heatmap.increment(result.start, result.stop - result.start);
            }
            Err(_) => {
                shuteye::sleep(shuteye::Timespec::from_nano(1000).unwrap());
            }
        }

        let now = time::precise_time_ns();

        if now - printed_at >= (config.duration as u64 * ONE_SECOND as u64) {
            if warmup {
                info!("-----");
                info!("Warmup complete");
                warmup = false;
                let _ = heatmap.clear();
            } else {
                let rate = (ONE_SECOND as u64 * (ok + miss)) as f64 / (now - printed_at) as f64;
                let mut success_rate = 0_f64;
                let mut hit_rate = 0_f64;
                if (histogram.entries() + error) > 0 {
                    success_rate = (100 * histogram.entries()) as f64 /
                                   (histogram.entries() + error) as f64;
                }
                if (hit + miss) > 0 {
                    hit_rate = (100 * hit) as f64 / (hit + miss) as f64;
                }
                info!("-----");
                info!("Window: {}", window);
                info!("Requests: {} Ok: {} Miss: {} Error: {} Closed: {}",
                      histogram.entries(),
                      ok,
                      miss,
                      error,
                      closed);
                info!("Rate: {:.*} rps Success: {:.*} % Hitrate: {:.*} %",
                      2,
                      rate,
                      2,
                      success_rate,
                      2,
                      hit_rate);
                info!("Latency: min: {} ns max: {} ns avg: {} ns stddev: {} ns",
                        histogram.minimum().unwrap_or(0),
                        histogram.maximum().unwrap_or(0),
                        histogram.mean().unwrap_or(0),
                        histogram.stddev().unwrap_or(0),
                    );
                info!("Percentiles: p50: {} ns p90: {} ns p99: {} ns p999: {} ns p9999: {} ns",
                        histogram.percentile(50.0).unwrap_or(0),
                        histogram.percentile(90.0).unwrap_or(0),
                        histogram.percentile(99.0).unwrap_or(0),
                        histogram.percentile(99.9).unwrap_or(0),
                        histogram.percentile(99.99).unwrap_or(0),
                    );
            }
            let _ = histogram.clear();
            ok = 0;
            hit = 0;
            miss = 0;
            error = 0;
            closed = 0;
            window += 1;
            printed_at = now;
            if window > config.windows {
                if let Some(file) = trace {
                    heatmap.save(file);
                }
                if let Some(file) = matches.opt_str("waterfall") {
                    let mut waterfall = Waterfall { heatmap: heatmap };
                    waterfall.render_png(file);
                }
                break;
            }
        }
    }
}
