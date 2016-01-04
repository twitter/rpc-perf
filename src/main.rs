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
                    _ => {
                        debug!("too many established connections")
                    }
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

    let _ = log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        return Box::new(SimpleLogger);
    });

    let mut opts = Options::new();

    opts.optopt("s", "server", "server address", "HOST:PORT");
    opts.optopt("c", "connections", "connections per thread", "INTEGER");
    opts.optopt("t", "threads", "number of threads", "INTEGER");
    opts.optopt("d", "duration", "duration of window in seconds", "INTERGER");
    opts.optopt("w", "windows", "number of windows in test", "INTEGER");
    opts.optopt("r", "rate", "global requests per second", "INTEGER");
    opts.optopt("p", "protocol", "client protocol", "STRING");
    opts.optopt("m", "command", "request command", "STRING");
    opts.optopt("b", "bytes", "value size in bytes", "INTEGER");
    opts.optopt("", "io_poll_timeout_ms", "io_poll_timeout_ms", "INTEGER");
    opts.optopt("", "notify_capacity", "notify_capacity", "INTEGER");
    opts.optopt("", "messages_per_tick", "messages_per_tick", "INTEGER");
    opts.optopt("", "timer_tick_ms", "timer_tick_ms", "INTEGER");
    opts.optopt("", "timer_wheel_size", "timer_wheel_size", "INTEGER");
    opts.optopt("", "timer_capacity", "timer_capacity", "INTEGER");
    opts.optopt("", "config", "TOML config file", "FILE");
    opts.optopt("", "trace", "write histogram data to file", "FILE");
    opts.optflag("", "nodelay", "enable tcp nodelay");
    opts.optflag("", "hit", "prepopulate key to get");
    opts.optflag("", "flush", "flush cache prior to test");
    opts.optflag("", "ipv4", "force IPv4 only");
    opts.optflag("", "ipv6", "force IPv6 only");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => {
            m
        }
        Err(f) => {
            panic!(f.to_string())
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if !matches.opt_present("s") {
        error!("require server parameter");
        print_usage(&program, opts);
        return;
    }
    let server = matches.opt_str("s").unwrap();
    let trace = matches.opt_str("trace");

    // start with the default config
    let mut c: BenchmarkConfig = Default::default();

    // load config from file if specified
    if matches.opt_present("config") {
        let toml = matches.opt_str("config").unwrap();

        c = config::load_config(toml).unwrap();
    }

    // override config with commandline options

    // these map to general section, and can override config
    if matches.opt_present("p") {
        c.protocol = matches.opt_str("p").unwrap();
    }
    if matches.opt_present("t") {
        match matches.opt_str("t").unwrap().parse() {
            Ok(threads) => {
                c.threads = threads;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "threads", e);
                return;
            }
        }
    }
    if matches.opt_present("c") {
        match matches.opt_str("c").unwrap().parse() {
            Ok(connections) => {
                c.connections = connections;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "connections", e);
                return;
            }
        }
    }
    if matches.opt_present("w") {
        match matches.opt_str("w").unwrap().parse() {
            Ok(windows) => {
                c.windows = windows;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "windows", e);
                return;
            }
        }
    }
    if matches.opt_present("d") {
        match matches.opt_str("d").unwrap().parse() {
            Ok(duration) => {
                c.duration = duration;
            }
            Err(e) => {
                error!("Bad parameter: {} Cause: {}", "duration", e);
                return;
            }
        }
    }
    if matches.opt_present("nodelay") {
        c.nodelay = true;
    }

    // these map to workload and conflict with config for simplicity
    if c.workloads.len() == 0 {
        let mut w: BenchmarkWorkload = Default::default();

        if matches.opt_present("r") {
            match matches.opt_str("r").unwrap().parse() {
                Ok(rate) => {
                    w.rate = rate;
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
                    w.bytes = bytes;
                }
                Err(e) => {
                    error!("Bad parameter: {} Cause: {}", "bytes", e);
                    return;
                }
            }
        }
        if matches.opt_present("m") {
            w.command = matches.opt_str("m").unwrap();
        }
        if matches.opt_present("flush") {
            w.flush = true;
        }
        if matches.opt_present("hit") {
            w.hit = true;
        }
        let _ = c.workloads.push(w);
    } else if matches.opt_present("r") || matches.opt_present("r") || matches.opt_present("b") ||
       matches.opt_present("m") {
        error!("workload is specified in config and commandline");
        print_usage(&program, opts);
        return;
    }

    // this stuff is pretty advanced, might be useless
    let mut io_poll_timeout_ms = 1_000;
    if matches.opt_present("io_poll_timeout_ms") {
        io_poll_timeout_ms = matches.opt_str("io_poll_timeout_ms").unwrap().parse().unwrap();
    }
    let mut notify_capacity = 4_096;
    if matches.opt_present("notify_capacity") {
        notify_capacity = matches.opt_str("notify_capacity").unwrap().parse().unwrap();
    }
    let mut messages_per_tick = 256;
    if matches.opt_present("messages_per_tick") {
        messages_per_tick = matches.opt_str("messages_per_tick").unwrap().parse().unwrap();
    }
    let mut timer_tick_ms = 100;
    if matches.opt_present("timer_tick_ms") {
        timer_tick_ms = matches.opt_str("timer_tick_ms").unwrap().parse().unwrap();
    }
    let mut timer_wheel_size = 1_024;
    if matches.opt_present("timer_wheel_size") {
        timer_wheel_size = matches.opt_str("timer_wheel_size").unwrap().parse().unwrap();
    }
    let mut timer_capacity = 65_536;
    if matches.opt_present("timer_capacity") {
        timer_capacity = matches.opt_str("timer_capacity").unwrap().parse().unwrap();
    }

    let workq = BoundedQueue::<Vec<u8>>::with_capacity(BUCKET_SIZE);

    let client_protocol: Protocol;

    match Protocol::new(&*c.protocol) {
        Ok(p) => {
            client_protocol = p;
        }
        Err(_) => {
            panic!("Bad protocol: {}", &*c.protocol);
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

    let evconfig = mio::EventLoopConfig {
        io_poll_timeout_ms: io_poll_timeout_ms,
        notify_capacity: notify_capacity,
        messages_per_tick: messages_per_tick,
        timer_tick_ms: timer_tick_ms,
        timer_wheel_size: timer_wheel_size,
        timer_capacity: timer_capacity,
    };

    info!("rpc-perf {} initializing...", VERSION);
    info!("-----");
    info!("Config:");
    info!("Config: Server: {} Protocol: {} IP: {:?}",
          server,
          c.protocol,
          internet_protocol);
    info!("Config: Threads: {} Connections: {}",
          c.threads,
          c.connections);
    info!("Config: Windows: {} Duration: {}", c.windows, c.duration);
    info!("-----");
    info!("Workload:");

    for i in 0..c.workloads.len() {
        let wl = c.workloads[i].clone();
        info!("Workload {}: Command: {} Bytes: {} Rate: {} Hit: {} Flush: {}",
              i,
              wl.command,
              wl.bytes,
              wl.rate,
              wl.hit,
              wl.flush);

        let mut workload = workload::Hotkey::new(i,
                                                 c.protocol.clone(),
                                                 wl.command,
                                                 wl.bytes,
                                                 wl.rate as u64,
                                                 workq.clone(),
                                                 1,
                                                 wl.hit,
                                                 wl.flush)
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
    for _ in 0..c.threads {
        let stats_tx = stats_tx.clone();
        let server = socket_addr.clone();
        let connections = c.connections.clone();
        let work_rx = workq.clone();
        let nodelay = c.nodelay.clone();
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
        if now - printed_at >= (c.duration as u64 * ONE_SECOND as u64) {
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
                                                       (window * c.duration),
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
            printed_at = now;
            window += 1;
            if window >= c.windows {
                break;
            }
        }
    }
}
