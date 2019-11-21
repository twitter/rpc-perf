// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! Metrics facade that allows for using multiple metrics backends.
//!
//! Metrics types should implement one of [`Counter`][counter],
//! [`Gauge`][gauge], or [`Summary`][summary]. Then they can be registered
//! through one of [`register_counter`][rctr], [`register_gauge`][rgauge], or
//! [`register_summary`][rhist] functions.
//!
//! # Metadata
//! Each individual metric can have metadata associated with. This is a set of
//! static key-value pairs that can be used to store arbitrary properties of the
//! metric. Empty metadata can be created by calling
//! [`Metadata::new`](Metadata::new). Otherwise, `Metadata` is created using the
//! `metadata` macro.
//!
//! # Introspection
//! To examine and query metrics, use the [`for_each_metric`][for_each_metric]
//! function.
//!
//! # Error Handling
//! This library has a somewhat idiosyncratic approach to error handling.
//! Instead of returning a result from each metric function/macro there
//! is a global error handling function (set by [`set_error_fn`](set_error_fn))
//! which is called with the error whenever an invalid action is performed.
//!
//! **Important Note:** attempting to record values to a non-existent metric
//! is not considered an error for performance reasons.
//!
//! # Example
//! ```rust
//! # use metrics_core::*;
//! # struct Metric;
//! # impl MetricCommon for Metric {}
//! # impl Summary for Metric {
//! #   fn record(&self, time: Instant, val: u64, count: u64) {}
//! #   fn submetrics(&self) -> Result<Vec<SubMetric>, SummaryError> { Ok(vec![]) }
//! #   fn quantiles(&self, q: &[Percentile], out: &mut [u64]) -> Result<(), SummaryError> { unimplemented!() }
//! #   fn buckets(&self) -> Result<Vec<Bucket>, SummaryError> { unimplemented!() }
//! # }
//! # fn function_that_takes_some_time() {}
//! // Create a metric named "example.metric" with no associated metadata
//! register_summary("example.metric", Box::new(Metric), Metadata::empty());
//!
//! // Alternatively, we can add metadata using the metadata! macro.
//! register_summary(
//!     "example.metadata",
//!     Box::new(Metric),
//!     metadata! {
//!         "some key" => "some value",
//!         "unit" => "ns"
//!     }
//! );
//!
//! // If you have a static reference to a metric, then you can avoid boxing it
//! static METRIC_INSTANCE: Metric = Metric;
//! register_summary(
//!     "example.static",
//!     &METRIC_INSTANCE,
//!     Metadata::empty()
//! );
//!
//! // Now we can use these metrics like so
//!
//! // Record single values to "example.metric"
//! value!("example.metric", 10);
//! value!("example.metric", 11);
//!
//! // Record a value and a count, note that the count
//! // is only used by summarys.
//! value!("example.static", 120, 44);
//!
//! // Can also record timings
//! let start = Instant::now();
//! function_that_takes_some_time();
//! let end = Instant::now();
//! // This gets translated to a duration in nanoseconds
//! interval!("example.metadata", start, end);
//! ```
//!
//! [counter]: crate::Counter
//! [gauge]: crate::Gauge
//! [summary]: crate::Summary
//! [rctr]: crate::register_counter
//! [rgauge]: crate::register_gauge
//! [rhist]: crate::register_summary
//! [for_each_metric]: crate::for_each_metric

#![warn(intra_doc_link_resolution_failure, missing_docs)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;

#[macro_use]
mod macros;

mod dyncow;
mod error;
mod inner;
mod instant;
mod metadata;
mod percentile;
mod scoped;
mod state;
mod submetric;
mod traits;
mod value;

pub use crate::dyncow::DynCow;
pub use crate::error::{
    MetricError, MetricErrorData, RegisterError, SummaryError, UnregisterError,
};
pub use crate::inner::{Metric, MetricInstance, MetricType};
pub use crate::instant::{Instant, Interval};
pub use crate::metadata::Metadata;
pub use crate::percentile::Percentile;
pub use crate::scoped::ScopedMetric;
pub use crate::submetric::{Bucket, SubMetric, SubMetricValue};
pub use crate::traits::{Counter, Gauge, MetricCommon, Summary};
pub use crate::value::MetricValue;

use std::borrow::Cow;

use crate::state::State;

/// Register a new counter.
///
/// If a metric has already been registered under the same name, then it will
/// return an error.
pub fn register_counter(
    name: impl Into<Cow<'static, str>>,
    counter: impl Into<DynCow<'static, dyn Counter>>,
    metadata: Metadata,
) -> Result<(), RegisterError> {
    State::get_force().register_metric(name.into(), Metric::Counter(counter.into()), metadata)
}

/// Register a new gauge.
///
/// If a metric has already been registered under the same name, then it will
/// return an error.
pub fn register_gauge(
    name: impl Into<Cow<'static, str>>,
    gauge: impl Into<DynCow<'static, dyn Gauge>>,
    metadata: Metadata,
) -> Result<(), RegisterError> {
    State::get_force().register_metric(name.into(), Metric::Gauge(gauge.into()), metadata)
}

/// Register a new summary.
///
/// If a metric has already been registered under the same name, then it will
/// return an error.
pub fn register_summary(
    name: impl Into<Cow<'static, str>>,
    summary: impl Into<DynCow<'static, dyn Summary>>,
    metadata: Metadata,
) -> Result<(), RegisterError> {
    State::get_force().register_metric(name.into(), Metric::Summary(summary.into()), metadata)
}

/// Unregister an existing metric.
///
/// If there is no such metric returns an error.
pub fn unregister_metric(name: impl AsRef<str>) -> Result<(), UnregisterError> {
    match State::get() {
        Some(state) => state.unregister_metric(name.as_ref()),
        None => Ok(()),
    }
}

/// Set the error function.
///
/// Due to the impracticality of having every single metric return a `Result`
/// this library instead opts to have an internal error function that is called
/// whenever an error occurs.
///
/// The default error function will log a warning when an error occurrs.
pub fn set_error_fn(err_fn: impl Fn(MetricError) + Send + Sync + 'static) {
    use std::sync::Arc;

    State::get_force().set_error_fn(Arc::new(err_fn));
}

/// Run a function over each metric and collect the result into a container.
///
/// Due to the underlying API limitations of evmap this is the only way to
/// introspect existing metrics.
pub fn for_each_metric<C, F, R>(func: F) -> C
where
    C: std::iter::FromIterator<R>,
    F: FnMut(&str, &MetricInstance) -> R,
{
    match State::get() {
        Some(state) => state.for_each_metric(func),
        None => C::from_iter(std::iter::empty()),
    }
}

#[doc(hidden)]
pub mod export {
    use super::*;

    pub fn create_metadata(attributes: &'static [(&'static str, &'static str)]) -> Metadata {
        Metadata::new(attributes)
    }

    pub fn current_time() -> Instant {
        Instant::now()
    }

    /// Record a value to a metric. This corresponds to the `value!` macro.
    #[inline]
    pub fn record_value(
        name: impl AsRef<str>,
        value: impl Into<MetricValue>,
        count: u64,
        time: Instant,
    ) {
        if let Some(state) = State::get() {
            state.record_value(name.as_ref(), value.into(), count, time);
        }
    }

    /// Record an increment to a counter or gauge. This corresponds to the
    /// `increment!` macro.
    #[inline]
    pub fn record_increment(name: impl AsRef<str>, amount: impl Into<MetricValue>, time: Instant) {
        if let Some(state) = State::get() {
            state.record_increment(name.as_ref(), amount.into(), time)
        }
    }

    /// Record a decrement to a gauge. This corresponds to the `decrement!`
    /// macro.
    #[inline]
    pub fn record_decrement(name: impl AsRef<str>, amount: impl Into<MetricValue>, time: Instant) {
        if let Some(state) = State::get() {
            state.record_decrement(name.as_ref(), amount.into(), time)
        }
    }

    /// Record a value, calls the error function if the metric is not a counter.
    #[inline]
    pub fn record_counter_value(name: impl AsRef<str>, amount: u64, time: Instant) {
        if let Some(state) = State::get() {
            state.record_counter_value(name.as_ref(), amount, time);
        }
    }

    /// Record a value, calls the error function if the metric is not a gauge.
    #[inline]
    pub fn record_gauge_value(name: impl AsRef<str>, amount: i64, time: Instant) {
        if let Some(state) = State::get() {
            state.record_gauge_value(name.as_ref(), amount, time);
        }
    }
}
