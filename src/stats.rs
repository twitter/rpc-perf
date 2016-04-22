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
extern crate heatmap;
extern crate histogram;
extern crate waterfall;
extern crate shuteye;

use std::sync::mpsc;

use heatmap::{Heatmap, HeatmapConfig};
use histogram::{Histogram, HistogramConfig};
use waterfall::Waterfall;

const ONE_MILISECOND: i64 = 1_000_000;
const ONE_SECOND: isize = 1_000_000_000;

#[derive(Clone)]
pub enum Status {
    Error,
    Hit,
    Miss,
    Ok,
    Closed,
}

#[derive(Clone)]
pub struct Stat {
    pub start: u64,
    pub stop: u64,
    pub status: Status,
}

pub struct Receiver {
    queue: mpsc::Receiver<Stat>,
}

impl Receiver {
    pub fn new(queue: mpsc::Receiver<Stat>) -> Receiver {
        Receiver { queue: queue }
    }

    pub fn run(&self,
               duration: usize,
               windows: usize,
               trace: Option<String>,
               waterfall: Option<String>) {
        let mut histogram_config = HistogramConfig::new();
        histogram_config.precision(4).max_value(60 * ONE_SECOND as u64);
        let mut histogram = Histogram::configured(histogram_config).unwrap();

        let mut heatmap_config = HeatmapConfig::new();
        heatmap_config.precision(2).max_value(ONE_SECOND as u64);
        heatmap_config.slice_duration(ONE_SECOND as u64).num_slices((duration * windows));
        let mut heatmap = Heatmap::configured(heatmap_config).unwrap();

        let mut printed_at = time::precise_time_ns();
        let mut ok = 0_u64;
        let mut hit = 0_u64;
        let mut miss = 0_u64;
        let mut error = 0_u64;
        let mut closed = 0_u64;
        let mut window = 0;
        let mut warmup = true;

        loop {
            match self.queue.try_recv() {
                Ok(result) => {
                    match result.status {
                        Status::Ok => {
                            ok += 1;
                        }
                        Status::Hit => {
                            hit += 1;
                            ok += 1;
                        }
                        Status::Miss => {
                            miss += 1;
                            ok += 1;
                        }
                        Status::Error => {
                            error += 1;
                        }
                        Status::Closed => {
                            closed += 1;
                        }
                    }
                    let _ = histogram.increment(result.stop - result.start);
                    let _ = heatmap.increment(result.start, result.stop - result.start);
                }
                Err(_) => {
                    trace!("stats queue empty");
                    shuteye::sleep(shuteye::Timespec::from_nano(ONE_MILISECOND).unwrap());
                }
            }

            let now = time::precise_time_ns();

            if now - printed_at >= (duration as u64 * ONE_SECOND as u64) {
                if warmup {
                    info!("-----");
                    info!("Warmup complete");
                    warmup = false;
                    let _ = heatmap.clear();
                } else {
                    let rate = (ONE_SECOND as u64 * (ok + miss)) as f64 / (now - printed_at) as f64;
                    let mut success_rate = 0_f64;
                    let mut hit_rate = 0_f64;
                    if (histogram.entries() + error) > 0 {
                        success_rate = (100 * ok) as f64 / (histogram.entries()) as f64;
                    }
                    if (hit + miss) > 0 {
                        hit_rate = (100 * hit) as f64 / (hit + miss) as f64;
                    }
                    info!("-----");
                    info!("Window: {}", window);
                    info!("Requests: {} Ok: {} Miss: {} Error: {} Closed: {}",
                          histogram.entries(),
                          ok,
                          miss,
                          error,
                          closed);
                    info!("Rate: {:.*} rps Success: {:.*} % Hitrate: {:.*} %",
                          2,
                          rate,
                          2,
                          success_rate,
                          2,
                          hit_rate);
                    info!("Latency: min: {} ns max: {} ns avg: {} ns stddev: {} ns",
	                        histogram.minimum().unwrap_or(0),
	                        histogram.maximum().unwrap_or(0),
	                        histogram.mean().unwrap_or(0),
	                        histogram.stddev().unwrap_or(0),
	                    );
                    info!("Percentiles: p50: {} ns p90: {} ns p99: {} ns p999: {} ns p9999: {} ns",
	                        histogram.percentile(50.0).unwrap_or(0),
	                        histogram.percentile(90.0).unwrap_or(0),
	                        histogram.percentile(99.0).unwrap_or(0),
	                        histogram.percentile(99.9).unwrap_or(0),
	                        histogram.percentile(99.99).unwrap_or(0),
	                    );
                }
                let _ = histogram.clear();
                ok = 0;
                hit = 0;
                miss = 0;
                error = 0;
                closed = 0;
                window += 1;
                printed_at = now;
                if window > windows {
                    if let Some(file) = trace {
                        debug!("saving heatmap trace file");
                        heatmap.save(file);
                    }
                    if let Some(file) = waterfall {
                        debug!("saving waterfall png");
                        let mut waterfall = Waterfall { heatmap: heatmap };
                        waterfall.render_png(file);
                    }
                    break;
                }
            }
        }
    }
}
