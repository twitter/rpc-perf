// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#![allow(unused_imports)]
use datastructures::*;
use logger::*;
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};

use std::collections::HashMap;

fn main() {
    Logger::new()
        .label("simulator")
        .level(Level::Debug)
        .init()
        .expect("Failed to initialize logger");

    info!("Welcome to the simulator!");

    let histogram = Histogram::<AtomicU64>::new(1_000_000, 2, None, None);
    let heatmap = Heatmap::<AtomicU64>::new(1_000_000, 2, 1_000_000, 5_000_000_000);

    let distribution = Normal::new(500.0, 250.0).unwrap();

    let start = std::time::Instant::now();

    loop {
        let now = std::time::Instant::now();
        if now - start >= std::time::Duration::new(5, 0) {
            break;
        }
        if now - start >= std::time::Duration::new(0, 1_000_000) {
            heatmap.latch();
        }
        let value: f64 = distribution.sample(&mut thread_rng());
        let value = value.floor() as u64;
        histogram.increment(value, 1);
        heatmap.increment(time::precise_time_ns(), value, 1);
    }

    info!(
        "data: samples: {} too_high: {} mean: {:?} mode: {:?}",
        histogram.total_count(),
        histogram.too_high(),
        histogram.mean(),
        histogram.mode(),
    );
    let mut labels = HashMap::new();
    labels.insert(0, "0".to_string());
    labels.insert(100, "100".to_string());
    labels.insert(1000, "1000".to_string());
    labels.insert(10000, "10000".to_string());
    labels.insert(100000, "100000".to_string());
    waterfall::save_waterfall(&heatmap, "waterfall.png", labels, 1_000_000_000);
}
