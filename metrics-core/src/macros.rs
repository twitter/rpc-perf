// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

/// Record a value to a metric.
///
/// For counters and gauges this should directly set the value of the metric,
/// for histograms it will get rolled into a summary.
///
/// The required parameters for this macro are
/// - `name`: a string literal with the name of the metric
/// - `value`: the value to be recorded
///
/// Optional parameters
/// - `count`: The number of times the value should be registered. This is
///     ignored for counters and gauges. If not given, this defaults to 1.
/// - `time`: The time at which the value was recorded, given as
///     `time = <expr>`. If not given then defaults to the current time.
///
/// # Example
/// ```
/// use metrics_core::{value, Instant};
///
/// // Set the value of the metric with the current timestamp
/// value!("my-metric", 15);
///
/// // Set the value of the metrics 16 times with the current timestamp.
/// // This is only different from the first one if "my-metric" is a
/// // summary. Otherwise, it has the exact same behaviour.
/// value!("my-metric", 15, 16);
///
/// // Set the value of the metric with an explicit time. Normally this
/// // isn't necessary but it is useful if you want to avoid getting
/// // the current time multiple times when setting a number of metrics.
/// let now = Instant::now();
/// value!("my-metric-1", 15, time = now);
/// value!("my-metric-2", 727, time = now);
/// value!("my-metric-3", 222, 23, time = now);
/// ```
#[macro_export]
macro_rules! value {
    ($name:literal, $value:expr) => {
        value!($name, $value, 1)
    };
    ($name:literal, $value:expr, time = $time:expr) => {
        value!($name, $value, 1, time = $time)
    };
    ($name:literal, $value:expr, $count:expr) => {
        value!($name, $value, $count, time = $crate::export::current_time())
    };
    ($name:literal, $value:expr, $count:expr, time=$time:expr) => {
        $crate::export::record_value($name, $value, $count, $time)
    };
}

/// Increment a counter or gauge.
///
/// If the metric is a counter or a gauge then it will increment the stored
/// value within the metric. If the provided metric is not a histogram, then it
/// will call the user-provided error function.
///
/// The the only required parameter for this macro is
/// - `name`: a string literal with the name of the metric.
///
/// Optional Paramters
/// - `value`: the amount by which to increment the counter/gauge. If not
///     specified this defaults to `1`.
/// - `time`: The time at which the increment happened. If not specified this
///     defaults to the current time. Specified like `time = <expr>`.
///
/// # Example
/// ```
/// use metrics_core::{increment, Instant};
///
/// // Increase the value of "my-metric" by 1.
/// increment!("my-metric");
///
/// // Increment the metric with an explicit time. Normally this isn't
/// // necessary but it is useful if you want to avoid getting the current
/// // time multiple times when incrementing multiple metrics.
/// let now = Instant::now();
/// increment!("my-metric-1", 15, time = now);
/// increment!("my-metric-2", time = now);
/// ```
#[macro_export]
macro_rules! increment {
    ($name:literal) => {
        increment!($name, 1)
    };
    ($name:literal, time = $time:expr) => {
        increment!($name, 1, time = $time)
    };
    ($name:literal, $value:expr) => {
        increment!($name, $value, time = $crate::export::current_time())
    };
    ($name:literal, $value:expr, time = $time:expr) => {
        $crate::export::record_increment($name, $value, $time)
    };
}

/// Decrement a gauge.
///
/// If the metric is a gauge then it will decrement the stored value within the
/// metric. If the provided metric is not a gauge, then it will call the
/// user-provided error function.
///
/// The the only required parameter for this macro is
/// - `name`: a string literal with the name of the metric.
///
/// Optional Paramters
/// - `value`: the amount by which to decrement the gauge.
///     If not specified this defaults to `1`.
/// - `time`: The time at which the decrement happened. If not specified
///     this defaults to the current time. Specified like `time = <expr>`.
///
/// # Example
/// ```
/// use metrics_core::{decrement, Instant};
///
/// // Decrease the value of "my-metric" by 1.
/// decrement!("my-metric");
///
/// // Decrement the metric with an explicit time. Normally this isn't
/// // necessary but it is useful if you want to avoid getting the current
/// // time multiple times when incrementing multiple metrics.
/// let now = Instant::now();
/// decrement!("my-metric-1", 15, time = now);
/// decrement!("my-metric-2", time = now);
/// ```
#[macro_export]
macro_rules! decrement {
    ($name:literal) => {
        decrement!($name, 1)
    };
    ($name:literal, time = $time:expr) => {
        decrement!($name, 1, time = $time)
    };
    ($name:literal, $value:expr) => {
        decrement!($name, $value, time = $crate::export::current_time())
    };
    ($name:literal, $value:expr, time = $time:expr) => {
        $crate::export::record_decrement($name, $value, $time)
    };
}

/// Set the value of a counter.
///
/// If the metric is not a counter then it will call the user-defined error
/// function.
///
/// ## Parameters
/// - `name`: A string literal with the name of the metric.
/// - `value`: The new value of the counter.
/// - `time`: The time at which the value was recorded.
///   Specified like `time = <expr>`.
///
/// # Example
/// ```
/// # use metrics_core::counter;
/// // Set the value of "my-metric" only if it is a counter and call the
/// // error function otherwise.
/// counter!("my-metric", 33);
/// ```
#[macro_export]
macro_rules! counter {
    ($name:literal, $value:expr) => {
        counter!($name, $value, time = $crate::export::current_time())
    };
    ($name:literal, $value:expr, time = $time:expr) => {
        $crate::export::record_counter_value($name, $value, $time)
    };
}

/// Set the value of a gauge.
///
/// If the metric is not a gauge then it will call the user-defined error
/// function.
///
/// ## Parameters
/// - `name`: A string literal with the name of the metric.
/// - `value`: The new value of the gauge.
/// - `time`: The time at which the value was recorded.
///   Specified like `time = <expr>`.
///
/// # Example
/// ```
/// # use metrics_core::gauge;
/// // Set the value of "my-metric" only if it is a counter and call the
/// // error function otherwise.
/// gauge!("my-metric", 33);
/// ```
#[macro_export]
macro_rules! gauge {
    ($name:literal, $value:expr) => {
        gauge!($name, $value, time = $crate::export::current_time())
    };
    ($name:literal, $value:expr, time = $time:expr) => {
        $crate::export::record_gauge_value($name, $value, $time)
    };
}

/// Record a timing interval.
///
/// This is equivalent to calling `value!` with the interval.
///
/// This macro supports two argument formats. Either it takes the duration the
/// interval or it takes a start and end time and uses that to calculate the
/// interval duration.
///
/// # Example
/// ```
/// # fn some_fn() {}
/// use metrics_core::{interval, Interval, Instant};
///
/// // Record an interval of length 30ms ending at the current time
/// interval!("my-metric", Interval::from_millis(30));
///
/// // Time some_fn and record it
/// let start = Instant::now();
/// some_fn();
/// let end = Instant::now();
/// interval!("some_fn_length", start, end);
/// ```
#[macro_export]
macro_rules! interval {
    ($name:literal, $value:expr) => {
        $crate::export::record_value($name, $value, 1, $crate::export::current_time())
    };
    ($name:literal, $start:expr, $end:expr) => {
        $crate::export::record_value($name, $start - $end, 1, $end)
    };
}

/// Register a new counter metric.
///
/// ## Parameters
/// - `name`: A string name that the metric will be registered under.
/// - `counter`: The actual counter metric.
/// - `metadata`: Arbitrary metadata associated with the metric. See the
///     [`metadata!`](crate::metadata) macro for the syntax.
#[macro_export]
macro_rules! register_counter {
    (
        $name:expr,
        $counter:expr
        $( ,
            { $( $key:ident : $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_counter!($name, $counter, $crate::metadata! {
            $( $key : $val ),*
        })
    };
    (
        $name:expr,
        $counter:expr,
        $( ,
            { $( $key:expr => $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_counter!($name, $counter, $crate::metadata! {
            $( $key => $val ),*
        })
    };
    (
        $name:expr,
        $counter:expr,
        $metadata:expr $(,)?
    ) => {
        $crate::register_counter(
            $name,
            $counter,
            $metadata
        )
    }
}

/// Register a new gauge metric
///
/// ## Parameters
/// - `name`: A string name that the metric will be registered under.
/// - `gauge`: The actual gauge metric.
/// - `metadata`: Arbitrary metadata associated with the metric. See the
///     [`metadata!`](crate::metadata) macro for the syntax.
#[macro_export]
macro_rules! register_gauge {
    (
        $name:expr,
        $gauge:expr
        $( ,
            { $( $key:tt : $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_gauge!($name, $gauge, $crate::metadata! {
            $( $key : $val ),*
        })
    };
    (
        $name:expr,
        $gauge:expr,
        $( ,
            { $( $key:expr => $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_gauge!($name, $gauge, $crate::metadata! {
            $( $key => $val ),*
        })
    };
    (
        $name:expr,
        $gauge:expr,
        $metadata:expr $(,)?
    ) => {
        $crate::register_gauge(
            $name,
            $counter,
            $metadata
        )
    }
}

/// Register a new summary.
///
/// ## Parameters
/// - `name`: A string name that the metric will be registered under.
/// - `summary`: The actual summary metric.
/// - `metadata`: Arbitrary metadata associated with the metric. See the
///     [`metadata!`](crate::metadata) macro for the syntax.
#[macro_export]
macro_rules! register_summary {
    (
        $name:expr,
        $histogram:expr
        $( ,
            { $( $key:tt : $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_histogram!($name, $histogram, $crate::metadata! {
            $( $key : $val ),*
        })
    };
    (
        $name:expr,
        $histogram:expr,
        $( ,
            { $( $key:expr => $val:expr ),* $(,)? }
        )? $(,)?
    ) => {
        register_counter!($name, $histogram, $crate::metadata! {
            $( $key => $val ),*
        })
    };
    (
        $name:expr,
        $histogram:expr,
        $metadata:expr $(,)?
    ) => {
        $crate::register_histogram(
            $name,
            $counter,
            $metadata
        )
    }
}

/// Create a metadata dictionary.
///
/// Metadata can contain arbitrary key-value pairs
/// as long as the keys and values are strings with
/// a static lifetime.
///
/// # Example
/// ```
/// # use metrics_core::*;
///
/// // Can use this syntax
/// let metdata = metadata! {
///     unit: "ms",
///     foo: "bar",
///     type: "histogram"
/// };
///
/// // Or this syntax.
/// let metadata = metadata! {
///     "unit" => "ms",
///     "foo" => "bar",
///     "type" => "histogram",
/// };
/// ```
#[macro_export]
macro_rules! metadata {
    {
        $( $key:ident : $val:expr ),* $(,)?
    } => {
        $crate::export::create_metadata(&[
            $( ( stringify!($key), $val ) ),*
        ])
    };
    {
        $( $key:expr => $val:expr ),* $(,)?
    } => {
        $crate::export::create_metadata(&[
            $( ( $key, $val ) ),*
        ])
    };
}

/// Create a percentile from a float.
///
/// This corresponds to calling `Percentile::from_float`.
#[macro_export]
macro_rules! percentile {
    ( $p:expr ) => {
        $crate::Percentile::from_float($p)
    };
}
