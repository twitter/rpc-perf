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

extern crate time;
extern crate log;

use common::padding::{Alignment, PadStr};
use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord};

pub struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Trace
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let ms = format!("{:.*}",
                             3,
                             ((time::precise_time_ns() % 1_000_000_000) / 1_000_000));
            let target = if record.metadata().level() >= LogLevel::Debug {
                record.target()
            } else {
                "rpc-perf"
            };
            println!("{}.{} {:<5} [{}] {}",
                     time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                     ms.pad(3, '0', Alignment::Right, true),
                     record.level().to_string(),
                     target,
                     record.args());
        }
    }
}

pub fn set_log_level(level: usize) {
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
