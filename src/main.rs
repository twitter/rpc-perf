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
extern crate time;
extern crate mio;
extern crate bytes;
extern crate histogram;
extern crate getopts;
extern crate regex;
extern crate parser;
extern crate shuteye;
extern crate request;
extern crate mpmc;
extern crate workload;
extern crate toml;

pub mod client;
pub mod config;
pub mod connection;
pub mod logger;
pub mod net;
pub mod state;
pub mod stats;

use getopts::Options;
use histogram::{Histogram, HistogramConfig};
use log::LogLevelFilter;
use mpmc::Queue as BoundedQueue;
use std::env;
use std::fs::File;
use std::io::prelude::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::thread;
use std::sync::mpsc;
use std::process;

use client::Client;
use config::{BenchmarkConfig, BenchmarkWorkload};
use connection::Connection;
use logger::SimpleLogger;
use net::InternetProtocol;
use parser::*;
use stats::*;
use workload::Protocol;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const ONE_SECOND: isize = 1_000_000_000;
const BUCKET_SIZE: usize = 10_000;

pub fn start(address: SocketAddr,
             connections: usize,
             stats_tx: mpsc::Sender<Stat>,
             protocol: Protocol,
             internet_protocol: InternetProtocol,
             work_rx: BoundedQueue<Vec<u8>>,
             nodelay: bool,
             config: mio::EventLoopConfig) {

    let mut event_loop = mio::EventLoop::configured(config).unwrap();

    let mut client = Client::new(work_rx.clone());

    let mut failures = 0;

    for _ in 0..connections {
        match net::to_mio_tcp_stream(address, internet_protocol) {
            Ok(stream) => {
                match client.connections.insert_with(|token| {
                    Connection::new(stream,
                                    token,
                                    stats_tx.clone(),
                                    protocol.clone(),
                                    nodelay.clone())
                }) {
                    Some(token) => {
                        event_loop.register_opt(&client.connections[token].socket,
                                                token,
                                                mio::EventSet::writable(),
                                                mio::PollOpt::edge() | mio::PollOpt::oneshot())
                                  .unwrap();
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
    info!("Connections: {} Failures: {}", connections, failures);
    if failures == connections {
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
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optopt("s", "server", "server address", "HOST:PORT");
    opts.optopt("t", "threads", "number of threads", "INTEGER");
    opts.optopt("c", "connections", "connections per thread", "INTEGER");
    opts.optopt("d", "duration", "number of seconds per window", "INTERGER");
    opts.optopt("w", "windows", "number of windows in test", "INTEGER");
    opts.optopt("r", "rate", "global requests per second", "INTEGER");
    opts.optopt("p", "protocol", "client protocol", "STRING");
    opts.optopt("m", "method", "request command", "STRING");
    opts.optopt("b", "bytes", "value size in bytes", "INTEGER");
    opts.optopt("", "config", "TOML config file", "FILE");
    opts.optopt("", "trace", "write histogram data to file", "FILE");
    opts.optflag("", "nodelay", "enable tcp nodelay");
    opts.optflag("", "hit", "prepopulate key to get");
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
        return Box::new(SimpleLogger);
    });

    // done manually to prevent getopts panic!()
    if !matches.opt_present("server") {
        error!("require server parameter");
        print_usage(&program, opts);
        return;
    }

    let server = matches.opt_str("server").unwrap();

    let trace = matches.opt_str("trace");

    // load config from file if specified
    if matches.opt_present("config") {
        let toml = matches.opt_str("config").unwrap();

        config = config::load_config(toml).unwrap();
    }

    // override config with commandline options

    // these map to general section, and can override config
    if matches.opt_present("protocol") {
        config.protocol = matches.opt_str("protocol").unwrap();
    }

    if matches.opt_present("threads") {
        match matches.opt_str("threads").unwrap().parse() {
            Ok(threads) => {
                config.threads = threads;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "threads", e);
                return;
            }
        }
    }

    if matches.opt_present("connections") {
        match matches.opt_str("connections").unwrap().parse() {
            Ok(connections) => {
                config.connections = connections;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "connections", e);
                return;
            }
        }
    }

    if matches.opt_present("windows") {
        match matches.opt_str("windows").unwrap().parse() {
            Ok(windows) => {
                config.windows = windows;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "windows", e);
                return;
            }
        }
    }

    if matches.opt_present("duration") {
        match matches.opt_str("duration").unwrap().parse() {
            Ok(duration) => {
                config.duration = duration;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "duration", e);
                return;
            }
        }
    }

    if matches.opt_present("nodelay") {
        config.nodelay = true;
    }

    // these map to workload and conflict with config for simplicity
    if config.workloads.len() == 0 {
        let mut workload: BenchmarkWorkload = Default::default();

        if matches.opt_present("rate") {
            match matches.opt_str("rate").unwrap().parse() {
                Ok(rate) => {
                    workload.rate = rate;
                }
                Err(e) => {
                    error!("Bad parameter: {} Cause: {}", "rate", e);
                    return;
                }
            }
        }

        if matches.opt_present("b") {
            match matches.opt_str("b").unwrap().parse() {
                Ok(bytes) => {
                    workload.bytes = bytes;
                }
                Err(e) => {
                    error!("Bad parameter: {} Cause: {}", "bytes", e);
                    return;
                }
            }
        }

        if matches.opt_present("m") {
            workload.command = matches.opt_str("m").unwrap();
        }

        if matches.opt_present("flush") {
            workload.flush = true;
        }

        if matches.opt_present("hit") {
            workload.hit = true;
        }

        let _ = config.workloads.push(workload);

    } else if matches.opt_present("rate") || matches.opt_present("bytes") ||
       matches.opt_present("method") {
        error!("workload is specified in config and commandline");
        print_usage(&program, opts);
        return;
    }

    let workq = BoundedQueue::<Vec<u8>>::with_capacity(BUCKET_SIZE);



    match Protocol::new(&*config.protocol) {
        Ok(p) => {
            client_protocol = p;
        }
        Err(_) => {
            panic!("Bad protocol: {}", &*config.protocol);
        }
    }

    let mut internet_protocol = InternetProtocol::Any;

    if matches.opt_present("ipv4") && matches.opt_present("ipv6") {
        error!("Use only --ipv4 or --ipv6");
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("ipv4") {
        internet_protocol = InternetProtocol::IpV4;
    }
    if matches.opt_present("ipv6") {
        internet_protocol = InternetProtocol::IpV6;
    }

    let evconfig = mio::EventLoopConfig::default();

    info!("rpc-perf {} initializing...", VERSION);
    info!("-----");
    info!("Config:");
    info!("Config: Server: {} Protocol: {} IP: {:?}",
          server,
          config.protocol,
          internet_protocol);
    info!("Config: Threads: {} Connections: {}",
          config.threads,
          config.connections);
    info!("Config: Windows: {} Duration: {}",
          config.windows,
          config.duration);
    info!("-----");
    info!("Workload:");

    for i in 0..config.workloads.len() {
        let w = config.workloads[i].clone();
        info!("Workload {}: Command: {} Bytes: {} Rate: {} Hit: {} Flush: {}",
              i,
              w.command,
              w.bytes,
              w.rate,
              w.hit,
              w.flush);

        let mut workload = workload::Hotkey::new(i,
                                                 config.protocol.clone(),
                                                 w.command,
                                                 w.bytes,
                                                 w.rate as u64,
                                                 workq.clone(),
                                                 1,
                                                 w.hit,
                                                 w.flush)
                               .unwrap();

        thread::spawn(move || {
            loop {
                workload.run();
            }
        });
    }

    let (stats_tx, stats_rx) = mpsc::channel();

    let socket_addr = &server.to_socket_addrs().unwrap().next().unwrap();

    info!("-----");
    info!("Connecting...");
    // spawn client threads
    for _ in 0..config.threads {
        let stats_tx = stats_tx.clone();
        let server = socket_addr.clone();
        let connections = config.connections.clone();
        let work_rx = workq.clone();
        let nodelay = config.nodelay.clone();
        let internet_protocol = internet_protocol.clone();

        thread::spawn(move || {
            start(server,
                  connections,
                  stats_tx,
                  client_protocol,
                  internet_protocol,
                  work_rx,
                  nodelay,
                  evconfig);
        });
    }

    let mut histogram_config = HistogramConfig::new();
    histogram_config.precision(4).max_value(ONE_SECOND as u64);
    let mut histogram = Histogram::configured(histogram_config).unwrap();

    let mut trace_file = File::open("/dev/null").unwrap();
    if trace.is_some() {
        trace_file = File::create(trace.clone().unwrap()).unwrap();
    }

    let mut printed_at = time::precise_time_ns();
    let mut ok = 0_u64;
    let mut hit = 0_u64;
    let mut miss = 0_u64;
    let mut error = 0_u64;
    let mut closed = 0_u64;
    let mut window = 0;

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
            }
            Err(_) => {
                shuteye::sleep(shuteye::Timespec::from_nano(1000).unwrap());
            }
        }
        let now = time::precise_time_ns();
        if now - printed_at >= (config.duration as u64 * ONE_SECOND as u64) {
            let rate = ONE_SECOND as u64 * (ok + miss) / (now - printed_at) as u64;
            let mut sr = 0;
            let mut hr = 0;
            if (histogram.entries() + error) > 0 {
                sr = 100 * histogram.entries() / (histogram.entries() + error);
            }
            if (hit + miss) > 0 {
                hr = 100 * hit / (hit + miss);
            }
            info!("-----");
            info!("Window: {}", (window + 1));
            info!("Requests: {} Ok: {} Miss: {} Error: {} Closed: {}",
                  histogram.entries(),
                  ok,
                  miss,
                  error,
                  closed);
            info!("Rate: {} rps Success: {} % Hitrate: {} %", rate, sr, hr);
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

            match trace {
                Some(..) => {
                    loop {
                        match histogram.next() {
                            Some(bucket) => {
                                if bucket.count() > 0 {
                                    let line = format!("{} {} {}\n",
                                                       (window * config.duration),
                                                       bucket.value(),
                                                       bucket.count())
                                                   .into_bytes();
                                    let _ = trace_file.write_all(&line);
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
                None => {}
            }

            let _ = histogram.clear();
            ok = 0;
            hit = 0;
            miss = 0;
            error = 0;
            closed = 0;
            window += 1;
            printed_at = now;
            if window >= config.windows {
                break;
            }
        }
    }
}
