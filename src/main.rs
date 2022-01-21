// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[macro_use]
extern crate rustcommon_logger;

use backtrace::Backtrace;
use clap::{App, Arg};
use rpc_perf::Builder;
use rustcommon_logger::{Level, Logger};

fn main() {
    // custom panic hook to terminate whole process after unwinding
    std::panic::set_hook(Box::new(|s| {
        error!("{}", s);
        println!("{:?}", Backtrace::new());
        std::process::exit(101);
    }));

    // parse command line options load configuration
    let matches = App::new(env!("CARGO_BIN_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .version_short("v")
        .long_about(
            "rpc-perf is used to generate synthetic traffic and measure the \
            performance characteristics of a server. It is primarily used to \
            evaluate the performance of cache backends and supports both \
            Memcached and Redis protocols.",
        )
        .about("Measure RPC performance using synthetic traffic")
        .arg(Arg::with_name("CONFIG").help("Configuration file").index(1))
        .get_matches();

    // initialize logging
    Logger::new()
        .label("rpc-perf")
        .level(Level::Info)
        .init()
        .expect("Failed to initialize logger");

    // launch
    Builder::new(matches.value_of("CONFIG")).spawn().wait()
}
