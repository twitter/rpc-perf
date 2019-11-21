// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::any::Any;

use crate::submetric::Bucket;
use crate::{Instant, Percentile, SubMetric, SummaryError};

/// Methods common to all metrics.
pub trait MetricCommon: Send + Sync {
    /// Get the current metric as a pointer to a type implementing `Any`.
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
}

// TODO(bmartin): consider making these generic in terms of primitive type.

/// A counter. Counts things.
///
/// This trait should be implemented by any type that should be used as a
/// counter metric.
///
/// Counters should be used when counting something. (e.g. the total number of
/// hits on a web endpoint, the number of times that a function has run, etc.)
pub trait Counter: MetricCommon {
    /// Set the value of the counter.
    fn store(&self, time: Instant, value: u64);

    /// Add a value to the counter.
    fn add(&self, time: Instant, value: u64);

    /// Get the current value of the counter.
    fn load(&self) -> u64;
}

/// A gauge. Measures the instantaneous value of some property.
///
/// This trait should be implemented by any type that can be used as a gauge
/// metric.
///
/// Gauges measure the instantaneous value of some property. (e.g. number of
/// requests currently in flight, current CPU usage, memory usage, etc.)
pub trait Gauge: MetricCommon {
    /// Store a value into the gauge.
    fn store(&self, time: Instant, value: i64);

    /// Add a value to the gauge.
    fn add(&self, time: Instant, value: i64);

    /// Subtract a value from the gauge.
    fn sub(&self, time: Instant, value: i64);

    /// Get the current value of the gauge.
    fn load(&self) -> u64;
}

/// Any sort of summary of the record values
pub trait Summary: MetricCommon {
    /// Record `count` instances of `value`.
    fn record(&self, time: Instant, value: u64, count: u64);

    // TODO: we should consider alternate design here. It's more likely that
    // we'd want to get some preset group of submetrics (eg: particular
    // quantiles) it's unclear how we'd tell the implementation about those in
    // the current form.

    /// Query quantiles from a summary.
    ///
    /// If this summary cannot produce quantiles, then it
    /// should return `SummaryError::Unsupported`.
    fn quantiles(&self, quantiles: &[Percentile], results: &mut [u64]) -> Result<(), SummaryError>;

    /// Query buckets from a summary.
    ///
    /// If this summary doesn't support buckets, then it
    /// should return `SummaryError::Unsupported`.
    fn buckets(&self) -> Result<Vec<Bucket>, SummaryError>;

    /// Get all custom statistics exposed by the implementation.
    ///
    /// If the summary doesn't export any custom statistics, then
    /// it should return `SummaryError::Unsupported`.
    fn submetrics(&self) -> Result<Vec<SubMetric>, SummaryError>;
}
