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


#[allow(unknown_lints, useless_attribute)]
#[cfg_attr(feature = "cargo-clippy", deny(result_unwrap_used))]
#[macro_use]
extern crate log;
extern crate log_panics;

extern crate bytes;
extern crate time;
extern crate rpcperf_request as request;
extern crate rpcperf_cfgtypes as cfgtypes;
extern crate rpcperf_common as common;
extern crate slab;

mod client;
mod connection;
mod logger;
mod net;
mod stats;


use client::Client;
use common::options::Options;
use common::stats::{Interest, Receiver, Stat};
use log::LogLevelFilter;
use logger::SimpleLogger;
use net::InternetProtocol;
use request::config;
use request::workload;
use std::env;
use std::thread;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn print_usage(program: &str, opts: &Options) {
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
    opts.optopt("",
                "request-timeout",
                "request timeout in milliseconds",
                "INTEGER");
    opts.optopt("",
                "connect-timeout",
                "connect timeout in milliseconds",
                "INTEGER");
    opts.optopt("p", "protocol", "client protocol", "STRING");
    opts.optopt("", "config", "TOML config file", "FILE");
    opts.optopt("", "listen", "listen address for stats", "HOST:PORT");
    opts.optopt("", "trace", "write histogram data to file", "FILE");
    opts.optopt("", "waterfall", "output waterfall PNG", "FILE");
    opts.optflag("", "tcp-nodelay", "enable tcp nodelay");
    opts.optflag("", "flush", "flush cache prior to test");
    opts.optflag("", "ipv4", "force IPv4 only");
    opts.optflag("", "ipv6", "force IPv6 only");
    opts.optflag("", "service", "run continuously");
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

#[allow(unknown_lints, cyclomatic_complexity)]
pub fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];
    let opts = opts();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            error!("Failed to parse command line args: {}", f);
            return;
        }
    };

    if matches.opt_present("help") {
        print_usage(program, &opts);
        return;
    }

    if matches.opt_present("version") {
        println!("rpc-perf {}", VERSION);
        return;
    }

    // initialize logging
    set_log_level(matches.opt_count("verbose"));
    log_panics::init();

    info!("rpc-perf {} initializing...", VERSION);
    if cfg!(feature = "asm") {
        info!("feature: asm: enabled");
    } else {
        info!("feature: asm: disabled");
    }

    if matches.opt_count("server") < 1 {
        error!("require server parameter");
        print_usage(program, &opts);
        return;
    }

    let listen = matches.opt_str("listen");
    let waterfall = matches.opt_str("waterfall");
    let trace = matches.opt_str("trace");


    // Load workload configuration
    let config = match config::load_config(&matches) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    let internet_protocol = match choose_layer_3(matches.opt_present("ipv4"),
                                                 matches.opt_present("ipv6")) {
        Ok(i) => i,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    info!("-----");
    info!("Config:");
    for server in matches.opt_strs("server") {
        info!("Config: Server: {} Protocol: {}",
              server,
              config.protocol_config.protocol.name());
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
    match config.timeout {
        Some(timeout) => {
            info!("Config: Request Timeout: {} ms", timeout);
        }
        None => {
            info!("Config: Request Timeout: None");
        }
    }
    match config.connect_timeout() {
        Some(timeout) => {
            info!("Config: Connect Timeout: {} ms", timeout);
        }
        None => {
            info!("Config: Connect Timeout: None");
        }
    }

    let mut stats_config = Receiver::<Stat>::configure()
        .batch_size(16)
        .capacity(65536)
        .duration(config.duration)
        .windows(config.windows);

    if let Some(addr) = listen {
        stats_config = stats_config.http_listen(addr);
    }



    let mut stats_receiver = stats_config.build();

    stats_receiver.add_interest(Interest::Count(Stat::Window));
    stats_receiver.add_interest(Interest::Count(Stat::ResponseOk));
    stats_receiver.add_interest(Interest::Count(Stat::ResponseOkHit));
    stats_receiver.add_interest(Interest::Count(Stat::ResponseOkMiss));
    stats_receiver.add_interest(Interest::Count(Stat::ResponseError));
    stats_receiver.add_interest(Interest::Count(Stat::ResponseTimeout));
    stats_receiver.add_interest(Interest::Count(Stat::RequestPrepared));
    stats_receiver.add_interest(Interest::Count(Stat::RequestSent));
    stats_receiver.add_interest(Interest::Count(Stat::ConnectOk));
    stats_receiver.add_interest(Interest::Count(Stat::ConnectError));
    stats_receiver.add_interest(Interest::Count(Stat::ConnectTimeout));
    stats_receiver.add_interest(Interest::Count(Stat::SocketCreate));
    stats_receiver.add_interest(Interest::Count(Stat::SocketClose));
    stats_receiver.add_interest(Interest::Count(Stat::SocketRead));
    stats_receiver.add_interest(Interest::Count(Stat::SocketWrite));
    stats_receiver.add_interest(Interest::Count(Stat::SocketFlush));

    stats_receiver.add_interest(Interest::Percentile(Stat::ResponseOk));
    stats_receiver.add_interest(Interest::Percentile(Stat::ResponseOkHit));
    stats_receiver.add_interest(Interest::Percentile(Stat::ResponseOkMiss));
    stats_receiver.add_interest(Interest::Percentile(Stat::ConnectOk));

    if let Some(w) = waterfall {
        stats_receiver.add_interest(Interest::Waterfall(Stat::ResponseOk, w));
    }

    if let Some(t) = trace {
        stats_receiver.add_interest(Interest::Waterfall(Stat::ResponseOk, t));
    }

    let mut send_queues = Vec::new();

    info!("-----");
    info!("Connecting...");
    for i in 0..config.threads {
        info!("Client: {}", i);

        let mut client_config = Client::configure();
        client_config
            .set_pool_size(config.connections)
            .stats(stats_receiver.get_sender().clone())
            .set_clocksource(stats_receiver.get_clocksource().clone())
            .set_protocol(config.protocol_config.protocol.clone())
            .set_request_timeout(config.timeout)
            .set_connect_timeout(config.connect_timeout())
            .set_internet_protocol(internet_protocol);

        for server in matches.opt_strs("server") {
            client_config.add_server(server);
        }

        let mut client = client_config.build();
        send_queues.push(client.tx());

        let _ = thread::Builder::new()
            .name(format!("client{}", i).to_string())
            .spawn(move || { client.run(); });
    }

    info!("-----");
    info!("Workload:");

    workload::launch_workloads(config.protocol_config.workloads,
                               send_queues.clone(),
                               stats_receiver.get_sender().clone(),
                               stats_receiver.get_clocksource().clone());

    stats::run(stats_receiver,
               config.windows,
               matches.opt_present("service"));
}
