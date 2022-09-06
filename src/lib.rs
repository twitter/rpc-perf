// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[macro_use]
extern crate rustcommon_logger;

#[macro_use]
mod macros;

mod admin;
// mod buffer;
mod codec;
mod config;
mod config_file;
mod metrics;
mod session;
mod time;
mod worker;

pub use crate::admin::Admin;
pub use crate::config::Config;
pub use crate::metrics::*;
pub use crate::session::{Session, TcpStream};
pub use crate::time::*;

use rustcommon_heatmap::{AtomicHeatmap, AtomicU64};
use rustcommon_logger::{File, LogBuilder, MultiLogBuilder, Output, Stdout};
use rustcommon_ratelimiter::Ratelimiter;

use std::sync::Arc;
use std::thread::JoinHandle;
use worker::Worker;

/// A structure which represents a runtime builder
pub struct Builder {
    admin: Admin,
    workers: Vec<Worker>,
}

impl Builder {
    /// Create a new runtime builder from the given config
    pub fn new(config: Option<&str>) -> Self {
        let config = Config::new(config);

        let config = Arc::new(config);

        let log_level = config.debug().log_level();

        let debug_output: Box<dyn Output> = if let Some(file) = config.debug().log_file() {
            let backup = config
                .debug()
                .log_backup()
                .unwrap_or(format!("{}.old", file));
            Box::new(
                File::new(&file, &backup, config.debug().log_max_size())
                    .expect("failed to open debug log file"),
            )
        } else {
            Box::new(Stdout::new())
        };

        let log = LogBuilder::new()
            .output(debug_output)
            .log_queue_depth(config.debug().log_queue_depth())
            .single_message_size(config.debug().log_single_message_size())
            .build()
            .expect("failed to initialize debug log");

        let log = MultiLogBuilder::new()
            .level_filter(log_level.to_level_filter())
            .default(log)
            .build()
            .start();

        let threads = config.general().threads() as u64;

        let connect_ratelimit = if let Some(r) = config.connection().ratelimit() {
            let r = Ratelimiter::new(threads, 1, r as u64);
            r.set_strategy(config.connection().ratelimit_model());
            Some(Arc::new(r))
        } else {
            None
        };

        let reconnect_ratelimit = if let Some(r) = config.connection().reconnect() {
            let r = Ratelimiter::new(threads, 1, r as u64);
            r.set_strategy(config.connection().ratelimit_model());
            Some(Arc::new(r))
        } else {
            None
        };

        let request_ratelimit = if let Some(r) = config.request().ratelimit() {
            let r = Ratelimiter::new(threads, 1, r as u64);
            r.set_strategy(config.request().ratelimit_model());
            Some(Arc::new(r))
        } else {
            None
        };

        let connect_heatmap = Some(Arc::new(AtomicHeatmap::<u64, AtomicU64>::new(
            1_000_000,
            3,
            Duration::from_secs(config.general().interval().as_secs()),
            Duration::from_millis(1000),
        )));

        let request_heatmap = Some(Arc::new(AtomicHeatmap::<u64, AtomicU64>::new(
            1_000_000,
            3,
            Duration::from_secs(config.general().interval().as_secs()),
            Duration::from_millis(1000),
        )));

        let request_waterfall = if config.waterfall().file().is_some() && config.general().windows().is_some() {
            Some(Arc::new(AtomicHeatmap::<u64, AtomicU64>::new(
                1_000_000_000,
                3,
                Duration::from_secs(config.general().interval().as_secs() * config.general().windows().unwrap() as u64),
                Duration::from_millis(1000),
            )))
        } else {
            None
        };

        for endpoint in config.endpoints() {
            info!("endpoint: {}", endpoint);
        }

        let mut workers = Vec::new();
        for _ in 0..threads {
            let mut worker = Worker::new(config.clone()).unwrap();
            worker.set_connect_ratelimit(connect_ratelimit.clone());
            worker.set_reconnect_ratelimit(reconnect_ratelimit.clone());
            worker.set_request_ratelimit(request_ratelimit.clone());
            worker.set_connect_heatmap(connect_heatmap.clone());
            worker.set_request_heatmap(request_heatmap.clone());
            worker.set_request_waterfall(request_waterfall.clone());
            workers.push(worker);
        }

        let mut admin = Admin::new(config, log);
        admin.set_connect_heatmap(connect_heatmap);
        admin.set_reconnect_ratelimit(reconnect_ratelimit);
        admin.set_request_heatmap(request_heatmap);
        admin.set_request_ratelimit(request_ratelimit);
        admin.set_request_waterfall(request_waterfall);

        Self { admin, workers }
    }

    /// Launch the runtime
    pub fn spawn(mut self) -> Runtime {
        let admin = self.admin;
        let admin_thread = std::thread::spawn(move || admin.run());

        let mut worker_threads = Vec::new();
        for mut worker in self.workers.drain(..) {
            worker_threads.push(std::thread::spawn(move || worker.run()));
        }

        Runtime {
            admin_thread,
            worker_threads,
        }
    }
}

#[allow(dead_code)]
/// Holds the runtime threads
pub struct Runtime {
    admin_thread: JoinHandle<()>,
    worker_threads: Vec<JoinHandle<()>>,
}

impl Runtime {
    /// Run the threads to completion
    pub fn wait(self) {
        let _ = self.admin_thread.join();
    }
}
