// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

extern crate metrics;

use metrics::*;
use std::{thread, time};
use std::sync::Arc;

pub const NS_PER_SEC: usize = 1_000_000_000;
pub const NS_PER_MINUTE: usize = 60 * NS_PER_SEC;

#[derive(Debug, Copy, Clone)]
pub enum MeasurementType {
    Counter,
    Distribution,
    Gauge,
    Increment,
    TimeInterval,
}

#[derive(Debug, Copy, Clone)]
pub enum TestStat {
    Counter(usize),
    Gauge(usize),
    Distribution(usize),
    TimeInterval(usize),
    Increment(usize),
}

impl Statistic for TestStat {
    fn name(&self) -> &str {
        match self {
            Self::Counter(v) | Self::Distribution(v) | Self::Gauge(v) | Self::TimeInterval(v) | Self::Increment(v) => {
                match v {
                    0 => "zero",
                    1 => "one",
                    2 => "two",
                    3 => "three",
                    4 => "four",
                    5 => "five",
                    6 => "six",
                    7 => "seven",
                    8 => "eight",
                    9 => "nine",
                    10 => "ten",
                    11 => "eleven",
                    12 => "twelve",
                    13 => "thirteen",
                    14 => "fourteen",
                    15 => "fifteen",
                    _ => "other",
                }
            }
        }
    }

    fn source(&self) -> Source {
        match self {
            Self::Counter(_) => Source::Counter,
            Self::Gauge(_) => Source::Gauge,
            Self::Distribution(_) => Source::Distribution,
            Self::TimeInterval(_) => Source::TimeInterval,
            Self::Increment(_) => Source::Counter,
        }
    }
}

pub fn main() {
    let runtime = 10.0;

    runner(
        runtime,
        MeasurementType::Counter,
        "Counter".to_string(),
    );
    runner(
        runtime,
        MeasurementType::Distribution,
        "Distribution".to_string(),
    );
    runner(
        runtime,
        MeasurementType::Gauge,
        "Gauge".to_string(),
    );
    runner(
        runtime,
        MeasurementType::Increment,
        "Increment".to_string(),
    );
    runner(
        runtime,
        MeasurementType::TimeInterval,
        "Time Interval".to_string(),
    );
}

pub fn runner(runtime: f64, measurement_type: MeasurementType, label: String) {
    for single_channel in [true, false].iter() {
        for i in [1, 2, 4, 8, 16, 32, 64].iter() {
            timed_run(
                *i,
                runtime,
                measurement_type,
                *single_channel,
                format!("{} (threads: {})", label, i),
            );
        }
    }
}

pub fn timed_run(
    threads: usize,
    runtime: f64,
    measurement_type: MeasurementType,
    single_channel: bool,
    label: String,
) {
    let max = 100_000;
    let duration = sized_run(threads, max, measurement_type, single_channel);
    let rate = max as f64 / duration;
    let max = (runtime * rate) as usize;
    let duration = sized_run(threads, max, measurement_type, single_channel);
    let rate = max as f64 / duration;
    println!(
        "{} (single channel: {}): {:.2e} updates/s",
        label, single_channel, rate
    );
}

pub fn sized_run(
    threads: usize,
    max: usize,
    measurement_type: MeasurementType,
    single_channel: bool,
) -> f64 {
    let metrics = Arc::new(Metrics::<AtomicU64>::new());
    let mut thread_pool = Vec::new();
    let t0 = time::Instant::now();
    for id in 0..threads {
        let metrics = metrics.clone();
        let id = if !single_channel {
            id
        } else {
            0
        };
        let statistic = match measurement_type {
            MeasurementType::Counter => {
                TestStat::Counter(id)
            }
            MeasurementType::Distribution => {
               TestStat::Distribution(id)
            }
            MeasurementType::Gauge => {
                TestStat::Gauge(id)
            }
            MeasurementType::Increment => {
                TestStat::Counter(id)
            }
            MeasurementType::TimeInterval => {
                TestStat::TimeInterval(id)
            }
        };
        metrics.register(&statistic, Some(Summary::Histogram(2_000_000_000, 3, None)));
        thread_pool.push(thread::spawn(move || {
            for value in 0..(max / threads) {
                match measurement_type {
                    MeasurementType::Counter => metrics.record_counter(&statistic, 1, value as u64),
                    MeasurementType::Distribution => metrics.record_distribution(&statistic, 1, value as u64, 1),
                    MeasurementType::Gauge => metrics.record_gauge(&statistic, 1, value as u64),
                    MeasurementType::Increment => metrics.record_increment(&statistic, 1, 1),
                    MeasurementType::TimeInterval => metrics.record_time_interval(&statistic, 1, value as u64),
                };
            }
        }));
    }
    for thread in thread_pool {
        thread.join().unwrap();
    }
    let t1 = time::Instant::now();
    (t1 - t0).as_secs() as f64 + ((t1 - t0).subsec_nanos() as f64 / NS_PER_SEC as f64)
}
