// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! A metrics library that provides for aggregation of measusurements taken
//! from counters, gauges, time-stamped events, external histograms, etc.
//!
//! # Goals
//! * efficient in terms of memory and cpu utilization
//! * flexible enough to serve multiple use-cases
//! * rich telemetry derived from simple measurements
//!
//! # Overview
//!
//! ## Metrics
//! This is the main structure which is used to add new `Channels`, record new
//! `Measurements`, and produce the `Readings` which were configured through
//! adding `Outputs` for various `Channels`. It contains all the data that has
//! been recorded.
//!
//! ## Channel
//! A `Channel` aggregates data from a specific source. A `Channel` can be
//! configured to track measurements taken from counters, distributions,
//! gauges, or time-intervals. The `Channel` allows for registering interest
//! in one or more `Output`s which are used to produce `Reading`s.
//!
//! ## Output
//! An `Output` is registered with a `Channel` to signal that a type of
//! `Reading` should be produced from the measurements recorded into that
//! `Channel`. Outputs can be counter readings, percentiles, or the time
//! offset of the min or max measurement.
//!
//! ## Reading
//! A `Reading` represents the value of a metric at a point in time. The
//! `Reading` stores information about the `Channel` label, the `Output` it
//! corresponds to, and the value.

#![deny(clippy::all)]

#[macro_use]
extern crate logger;

mod channel;
mod common;
mod metrics;

pub use crate::channel::Channel;
pub use crate::common::{Measurement, Output, Percentile, Point, Reading, Source};
pub use crate::metrics::Metrics;

pub use datastructures::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: u64, b: u64, precision: u64) -> bool {
        let power = 10_u64.pow(precision as u32) as f64;
        let log_a = (a as f64).log(power) as u64;
        let log_b = (b as f64).log(power) as u64;
        if (log_a + 1) >= log_b && log_a <= (log_b + 1) {
            println!("{} ~= {}", a, b);
            true
        } else {
            println!("{} !~= {}", a, b);
            false
        }
    }

    #[test]
    fn counter_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram_config = Histogram::<AtomicU64>::new(2_000_000_000, 3, None, None);
        metrics.add_channel(name.clone(), Source::Counter, Some(histogram_config));
        assert_eq!(metrics.counter("test".to_string()), 0);
        assert_eq!(metrics.percentile("test".to_string(), 0.0), None);
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 1_000_000_000,
                value: 1,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 1);
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 2_000_000_000,
                value: 1,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 1);
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 3_000_000_000,
                value: 2,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 2);
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 0.0).unwrap(),
            0,
            3
        ));
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 0.5).unwrap(),
            0,
            3
        ));
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 1.0).unwrap(),
            1,
            3
        ));
    }

    #[test]
    fn counter_wraparound() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram = Histogram::new(2_000_000_000, 3, None, None);
        metrics.add_channel(name.clone(), Source::Counter, Some(histogram));
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 0_u64.wrapping_sub(2_000_000_000),
                value: 0,
            },
        );
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 0_u64.wrapping_sub(1_000_000_000),
                value: 1,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 1);
        metrics.record(
            "test".to_string(),
            Measurement::Counter { time: 0, value: 2 },
        );
        assert_eq!(metrics.counter("test".to_string()), 2);
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 0.0).unwrap(),
            1,
            3
        ));
        metrics.zero();
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 0,
                value: 0_u64.wrapping_sub(1),
            },
        );
        metrics.record(
            "test".to_string(),
            Measurement::Counter {
                time: 1_000_000_000,
                value: 0,
            },
        );
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 0.0).unwrap(),
            1,
            3
        ));
    }

    #[test]
    fn counter_data() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram = Histogram::new(80_000_000_000, 3, None, None);
        metrics.add_channel(name.clone(), Source::Counter, Some(histogram));
        assert_eq!(metrics.counter("test".to_string()), 0);
        let data: Vec<u64> = vec![
            20334687810196614,
            20334700932559005,
            20334707934416079,
            20334715466281658,
            20334722865691396,
            20334729437570419,
            20334736349172794,
            20334744140066654,
            20334752014842899,
            20334759773262663,
            20334767739399083,
            20334776042704014,
            20334783846926280,
            20334792112381879,
            20334800539448702,
            20334806702815373,
            20334813358296654,
            20334821659085751,
            20334831578426342,
            20334840167485094,
            20334847154018880,
            20334855102223627,
            20334863614546286,
            20334872101854187,
            20334881347777697,
            20334889378069475,
            20334897879629869,
            20334907138339519,
            20334917775675515,
        ];
        for (time, &value) in data.iter().enumerate() {
            let time = time as u64 * 1_000_000_000;
            metrics.record("test".to_string(), Measurement::Counter { time, value });
            assert_eq!(metrics.counter("test".to_string()), value);
        }
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 0.0).unwrap(),
            6169999999,
            3
        ));
        assert!(approx_eq(
            metrics.percentile("test".to_string(), 1.0).unwrap(),
            13199999999,
            3
        ));
    }

    #[test]
    fn distribution_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram = Histogram::new(100, 3, None, None);
        metrics.add_channel(name.clone(), Source::Distribution, Some(histogram));
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::Distribution {
                value: 1,
                count: 1,
                time: 0,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 1);
        for i in 2..101 {
            metrics.record(
                "test".to_string(),
                Measurement::Distribution {
                    value: i,
                    count: 1,
                    time: 0,
                },
            );
        }
        assert_eq!(metrics.counter("test".to_string()), 100);
        assert_eq!(metrics.percentile("test".to_string(), 0.0), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.50), Some(50));
        assert_eq!(metrics.percentile("test".to_string(), 0.90), Some(90));
        assert_eq!(metrics.percentile("test".to_string(), 0.95), Some(95));
        assert_eq!(metrics.percentile("test".to_string(), 0.99), Some(99));
        assert_eq!(metrics.percentile("test".to_string(), 0.999), Some(100));
        assert_eq!(metrics.percentile("test".to_string(), 1.00), Some(100));
    }

    #[test]
    fn gauge_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram = Histogram::new(100, 3, None, None);
        metrics.add_channel(name.clone(), Source::Gauge, Some(histogram));
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record("test".to_string(), Measurement::Gauge { value: 0, time: 1 });
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::Gauge {
                value: 100,
                time: 1,
            },
        );
        assert_eq!(metrics.counter("test".to_string()), 100);
        metrics.record("test".to_string(), Measurement::Gauge { value: 0, time: 1 });
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::Gauge { value: 42, time: 1 },
        );
        assert_eq!(metrics.counter("test".to_string()), 42);
    }

    #[test]
    fn time_interval_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        let name = "test".to_string();
        let histogram = Histogram::new(100, 3, None, None);
        metrics.add_channel(name.clone(), Source::TimeInterval, Some(histogram));
        assert_eq!(metrics.counter("test".to_string()), 0);
        metrics.record(
            "test".to_string(),
            Measurement::TimeInterval { start: 0, stop: 1 },
        );
        assert_eq!(metrics.counter("test".to_string()), 1);
        for i in 1..100 {
            metrics.record(
                "test".to_string(),
                Measurement::TimeInterval {
                    start: i,
                    stop: i + 1,
                },
            );
        }
        assert_eq!(metrics.counter("test".to_string()), 100);
        assert_eq!(metrics.percentile("test".to_string(), 0.0), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.50), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.90), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.95), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.99), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 0.999), Some(1));
        assert_eq!(metrics.percentile("test".to_string(), 1.00), Some(1));
    }
}
