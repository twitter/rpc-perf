// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#![allow(unused_imports)]
use datastructures::*;
use rand::{thread_rng, Rng};
use rand_distr::*;
use rustcommon_logger::*;

use std::collections::HashMap;

pub const SECOND: u64 = 1_000_000_000;

fn main() {
    Logger::new()
        .label("simulator")
        .level(Level::Debug)
        .init()
        .expect("Failed to initialize logger");

    info!("Welcome to the simulator!");

    for shape in &[
        Shape::Cauchy,
        Shape::Normal,
        Shape::Uniform,
        Shape::Triangular,
        Shape::Gamma,
    ] {
        simulate(*shape);
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Cauchy,
    Normal,
    Uniform,
    Triangular,
    Gamma,
}

pub fn simulate(shape: Shape) {
    println!("simulating for {:?}", shape);
    let duration = 120;

    let heatmap = Heatmap::<AtomicU64>::new(SECOND, 3, SECOND, duration * SECOND);

    let cauchy = Cauchy::new(500_000.0, 2_000.00).unwrap();
    let normal = Normal::new(200_000.0, 100_000.0).unwrap();
    let uniform = Uniform::new_inclusive(10_000.0, 200_000.0);
    let triangular = Triangular::new(1.0, 2_000_000.0, 50_000.0).unwrap();
    let gamma = Gamma::new(2.0, 2.0).unwrap();

    let start = std::time::Instant::now();
    let mut latch = std::time::Instant::now();

    let mut rng = thread_rng();

    loop {
        let now = std::time::Instant::now();
        if now - start >= std::time::Duration::new(duration, 0) {
            break;
        }
        if now - latch >= std::time::Duration::new(1, 0) {
            heatmap.latch();
            latch = now;
        }
        let value: f64 = match shape {
            Shape::Cauchy => cauchy.sample(&mut rng),
            Shape::Normal => normal.sample(&mut rng),
            Shape::Uniform => uniform.sample(&mut rng),
            Shape::Triangular => triangular.sample(&mut rng),
            Shape::Gamma => gamma.sample(&mut rng) * 1_000_000.0,
        };
        let value = value.floor() as u64;
        heatmap.increment(time::precise_time_ns(), value, 1);
    }

    render(shape, heatmap);
}

pub fn render(shape: Shape, heatmap: Heatmap<AtomicU64>) {
    let mut labels = HashMap::new();
    labels.insert(100, "100ns".to_string());
    labels.insert(200, "200ns".to_string());
    labels.insert(400, "400ns".to_string());
    labels.insert(1_000, "1us".to_string());
    labels.insert(2_000, "2us".to_string());
    labels.insert(4_000, "4us".to_string());
    labels.insert(10_000, "10us".to_string());
    labels.insert(20_000, "20us".to_string());
    labels.insert(40_000, "40us".to_string());
    labels.insert(100_000, "100us".to_string());
    labels.insert(200_000, "200us".to_string());
    labels.insert(400_000, "400us".to_string());
    labels.insert(1_000_000, "1ms".to_string());
    labels.insert(2_000_000, "2ms".to_string());
    labels.insert(4_000_000, "4ms".to_string());
    labels.insert(10_000_000, "10ms".to_string());
    labels.insert(20_000_000, "20ms".to_string());
    labels.insert(40_000_000, "40ms".to_string());
    labels.insert(100_000_000, "100ms".to_string());
    labels.insert(200_000_000, "200ms".to_string());
    labels.insert(400_000_000, "400ms".to_string());

    let filename = match shape {
        Shape::Cauchy => "cauchy.png",
        Shape::Normal => "normal.png",
        Shape::Uniform => "uniform.png",
        Shape::Triangular => "tiangular.png",
        Shape::Gamma => "gamma.png",
    };

    waterfall::save_waterfall(&heatmap, filename, labels, 60 * SECOND);
}
