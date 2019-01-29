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

#![allow(unused_imports)]
use logger::*;
use datastructures::Histogram;
use datastructures::HeatmapBuilder;
use datastructures::histogram::Latched as FixedHistogram;
use rand::distributions::{Alphanumeric, Distribution, Gamma, LogNormal, Normal, Pareto, Uniform};
use rand::{thread_rng, Rng};
use std::collections::HashMap;

fn main() {
    Logger::new()
        .label("simulator")
        .level(Level::Debug)
        .init()
        .expect("Failed to initialize logger");

    info!("Welcome to the simulator!");

    let histogram = FixedHistogram::new(0, 1_000_000, 2);
    let heatmap = HeatmapBuilder::new(0, 1_000_000, 2, 1_000_000, 5_000_000_000).build();

    let distribution = Normal::new(500.0, 250.0);

    let start = std::time::Instant::now();

    loop {
        let now = std::time::Instant::now();
        if now - start >= std::time::Duration::new(5, 0) {
            break;
        }
        if now - start >= std::time::Duration::new(0, 1_000_000) {
            heatmap.latch();
        }
        let value = distribution.sample(&mut thread_rng()).abs();
        let value = value.floor() as usize;
        histogram.incr(value, 1);
        heatmap.incr(time::precise_time_ns() as usize, value, 1);
    }

    info!("data: samples: {} too_low: {} too_high: {} mean: {:?} mode: {:?} std_dev: {:?}",
        histogram.samples(),
        histogram.too_low(),
        histogram.too_high(),
        histogram.mean(),
        histogram.mode(),
        histogram.std_dev(),
    );
    let mut labels = HashMap::new();
    labels.insert(0, "0".to_string());
    labels.insert(100, "100".to_string());
    labels.insert(1000, "1000".to_string());
    labels.insert(10000, "10000".to_string());
    labels.insert(100000, "100000".to_string());
    waterfall::save_waterfall(&heatmap, "waterfall.png", labels, 1_000_000_000);
}
