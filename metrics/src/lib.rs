// Copyright 2019-2020 Twitter, Inc.
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

mod channel;
mod common;
mod metrics;

use crate::channel::Channel;
pub use crate::common::*;
pub use crate::metrics::Metrics;

pub use datastructures::*;

#[cfg(test)]
mod tests {
    use super::*;

    enum TestStat {
        Counter,
        Gauge,
        Distribution,
        TimeInterval,
    }

    impl Statistic for TestStat {
        fn name(&self) -> &str {
            match self {
                Self::Counter => "a",
                Self::Gauge => "b",
                Self::Distribution => "c",
                Self::TimeInterval => "d",
            }
        }

        fn source(&self) -> Source {
            match self {
                Self::Counter => Source::Counter,
                Self::Gauge => Source::Gauge,
                Self::Distribution => Source::Distribution,
                Self::TimeInterval => Source::TimeInterval,
            }
        }
    }

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
        metrics.register(
            &TestStat::Counter,
            Some(Summary::Histogram(2_000_000_000, 3, None)),
        );
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 0);
        assert_eq!(metrics.percentile(&TestStat::Counter, 0.0), None);
        metrics.record_counter(&TestStat::Counter, 1_000_000_000, 1);
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 1);
        metrics.record_counter(&TestStat::Counter, 2_000_000_000, 1);
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 1);
        metrics.record_counter(&TestStat::Counter, 3_000_000_000, 2);
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 2);
        assert!(approx_eq(
            metrics.percentile(&TestStat::Counter, 0.0).unwrap(),
            0,
            3
        ));
        assert!(approx_eq(
            metrics.percentile(&TestStat::Counter, 0.5).unwrap(),
            0,
            3
        ));
        assert!(approx_eq(
            metrics.percentile(&TestStat::Counter, 1.0).unwrap(),
            1,
            3
        ));
    }

    #[test]
    fn counter_wraparound() {
        let metrics = Metrics::<AtomicU64>::new();
        metrics.register(
            &TestStat::Counter,
            Some(Summary::Histogram(2_000_000_000, 3, None)),
        );
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 0);
        metrics.record_counter(&TestStat::Counter, 0_u64.wrapping_sub(2_000_000_000), 0);
        metrics.record_counter(&TestStat::Counter, 0_u64.wrapping_sub(1_000_000_000), 1);
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 1);
        metrics.record_counter(&TestStat::Counter, 0, 2);
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 2);
        assert!(approx_eq(
            metrics.percentile(&TestStat::Counter, 0.0).unwrap(),
            1,
            3
        ));
        metrics.zero();
        assert_eq!(metrics.reading(&TestStat::Counter).unwrap(), 0);
        metrics.record_counter(&TestStat::Counter, 0, 0_u64.wrapping_sub(1));
        metrics.record_counter(&TestStat::Counter, 1_000_000_000, 0);
        assert!(approx_eq(
            metrics.percentile(&TestStat::Counter, 0.0).unwrap(),
            1,
            3
        ));
    }

    #[test]
    fn distribution_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        metrics.register(
            &TestStat::Distribution,
            Some(Summary::Histogram(2_000_000_000, 3, None)),
        );
        assert_eq!(metrics.reading(&TestStat::Distribution).unwrap(), 0);
        metrics.record_distribution(&TestStat::Distribution, 0, 1, 1);
        assert_eq!(metrics.reading(&TestStat::Distribution).unwrap(), 1);
        for i in 2..101 {
            metrics.record_distribution(&TestStat::Distribution, 0, i, 1);
        }
        assert_eq!(metrics.reading(&TestStat::Distribution).unwrap(), 100);
        assert_eq!(metrics.percentile(&TestStat::Distribution, 0.0), Some(1));
        assert_eq!(metrics.percentile(&TestStat::Distribution, 0.50), Some(50));
        assert_eq!(metrics.percentile(&TestStat::Distribution, 0.90), Some(90));
        assert_eq!(metrics.percentile(&TestStat::Distribution, 0.95), Some(95));
        assert_eq!(metrics.percentile(&TestStat::Distribution, 0.99), Some(99));
        assert_eq!(
            metrics.percentile(&TestStat::Distribution, 0.999),
            Some(100)
        );
        assert_eq!(metrics.percentile(&TestStat::Distribution, 1.00), Some(100));
    }

    #[test]
    fn gauge_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        metrics.register(
            &TestStat::Gauge,
            Some(Summary::Histogram(2_000_000_000, 3, None)),
        );
        assert_eq!(metrics.reading(&TestStat::Gauge).unwrap(), 0);
        metrics.record_gauge(&TestStat::Gauge, 1, 0);
        assert_eq!(metrics.reading(&TestStat::Gauge).unwrap(), 0);
        metrics.record_gauge(&TestStat::Gauge, 1, 100);
        assert_eq!(metrics.reading(&TestStat::Gauge).unwrap(), 100);
        metrics.record_gauge(&TestStat::Gauge, 1, 0);
        assert_eq!(metrics.reading(&TestStat::Gauge).unwrap(), 0);
        metrics.record_gauge(&TestStat::Gauge, 1, 42);
        assert_eq!(metrics.reading(&TestStat::Gauge).unwrap(), 42);
    }

    #[test]
    fn time_interval_channel() {
        let metrics = Metrics::<AtomicU64>::new();
        metrics.register(
            &TestStat::TimeInterval,
            Some(Summary::Histogram(2_000_000_000, 3, None)),
        );
        assert_eq!(metrics.reading(&TestStat::TimeInterval).unwrap(), 0);
        metrics.record_time_interval(&TestStat::TimeInterval, 0, 1);
        assert_eq!(metrics.reading(&TestStat::TimeInterval).unwrap(), 1);
        for i in 1..100 {
            metrics.record_time_interval(&TestStat::TimeInterval, i, i + 1);
        }
        assert_eq!(metrics.reading(&TestStat::TimeInterval).unwrap(), 100);
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.0), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.50), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.90), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.95), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.99), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 0.999), Some(1));
        assert_eq!(metrics.percentile(&TestStat::TimeInterval, 1.00), Some(1));
    }
}
