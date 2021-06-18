// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rpc_perf::Builder;

use rustcommon_logger::{Level, Logger};

fn main() {
    // initialize logging
    Logger::new()
        .label("rpc-perf")
        .level(Level::Debug)
        .init()
        .expect("Failed to initialize logger");

    // launch
    Builder::new(std::env::args().nth(1)).spawn().wait()
}
