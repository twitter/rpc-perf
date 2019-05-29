//  Copyright 2019 Twitter, Inc
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

extern crate metrics;

use metrics::*;
use std::{thread, time};

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

pub fn main() {
    let runtime = 10.0;

    runner(
        runtime,
        Source::Counter,
        MeasurementType::Counter,
        "Counter".to_string(),
    );
    runner(
        runtime,
        Source::Distribution,
        MeasurementType::Distribution,
        "Distribution".to_string(),
    );
    runner(
        runtime,
        Source::Gauge,
        MeasurementType::Gauge,
        "Gauge".to_string(),
    );
    runner(
        runtime,
        Source::Counter,
        MeasurementType::Increment,
        "Increment".to_string(),
    );
    runner(
        runtime,
        Source::TimeInterval,
        MeasurementType::TimeInterval,
        "Time Interval".to_string(),
    );
}

pub fn runner(runtime: f64, source: Source, measurement_type: MeasurementType, label: String) {
    for single_channel in [true, false].iter() {
        for i in [1, 2, 4, 8, 16, 32, 64].iter() {
            timed_run(
                *i,
                runtime,
                source,
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
    source: Source,
    measurement_type: MeasurementType,
    single_channel: bool,
    label: String,
) {
    let max = 100_000;
    let duration = sized_run(threads, max, source, measurement_type, single_channel);
    let rate = max as f64 / duration;
    let max = (runtime * rate) as usize;
    let duration = sized_run(threads, max, source, measurement_type, single_channel);
    let rate = max as f64 / duration;
    println!(
        "{} (single channel: {}): {:.2e} updates/s",
        label, single_channel, rate
    );
}

pub fn sized_run(
    threads: usize,
    max: usize,
    source: Source,
    measurement_type: MeasurementType,
    single_channel: bool,
) -> f64 {
    let recorder = Recorder::<u64>::new();

    let mut thread_pool = Vec::new();
    let t0 = time::Instant::now();
    for tid in 0..threads {
        let recorder = recorder.clone();
        let label = if !single_channel {
            format!("test{}", tid)
        } else {
            "test".to_string()
        };
        let histogram_config = HistogramBuilder::new(2_000_000_000, 3, None, None);
        recorder.add_channel(label.clone(), source, Some(histogram_config));
        thread_pool.push(thread::spawn(move || {
            for value in 0..(max / threads) {
                let measurement = match measurement_type {
                    MeasurementType::Counter => Measurement::Counter {
                        time: value as u64,
                        value: value as u64,
                    },
                    MeasurementType::Distribution => Measurement::Distribution {
                        value: value as u64,
                        count: 1,
                        time: 1,
                    },
                    MeasurementType::Gauge => Measurement::Gauge {
                        value: value as u64,
                        time: 1,
                    },
                    MeasurementType::Increment => Measurement::Increment { count: 1, time: 1 },
                    MeasurementType::TimeInterval => Measurement::TimeInterval {
                        start: 1,
                        stop: value as u64,
                    },
                };
                recorder.record(label.clone(), measurement);
            }
        }));
    }
    for thread in thread_pool {
        thread.join().unwrap();
    }
    let t1 = time::Instant::now();
    (t1 - t0).as_secs() as f64 + ((t1 - t0).subsec_nanos() as f64 / NS_PER_SEC as f64)
}
