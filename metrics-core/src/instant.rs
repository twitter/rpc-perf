// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::ops::Sub;

use time;

/// High-resolution timestamp.
///
/// This timestamp behaves in most respects like
/// [`std::time::Instant`](std::time::Instant). However, when subtracting two
/// instants `A - B` it will return a duration of zero when `A` is before `B`.
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Instant {
    // Our representation is the number of nanoseconds since an unspecified
    // (but consistent within the process) epoch. For more details see the docs
    // for the time crate.
    //
    // In practice this is the return value of `time::precise_time_ns`
    ns_since_epoch: u64,
}

impl Instant {
    /// Get the current time
    pub fn now() -> Self {
        Instant {
            ns_since_epoch: time::precise_time_ns(),
        }
    }
}

/// Timespan between two instance.
///
/// Similar to [`std::time::Duration`](std::time::Duration).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Interval {
    duration_ns: u64,
}

impl Interval {
    /// Convert a number of seconds to an interval.
    ///
    /// Saturates to the maximum value if secs is more than `u64::MAX`
    /// nanoseconds.
    pub fn from_seconds(secs: u64) -> Self {
        Self {
            duration_ns: secs.saturating_mul(1_000_000_000),
        }
    }

    /// Convert a number of milliseconds to an interval.
    ///
    /// Saturates to the maximum value if secs is more than `u64::MAX`
    /// nanoseconds.
    pub fn from_millis(millis: u64) -> Self {
        Self {
            duration_ns: millis.saturating_mul(1_000_000),
        }
    }

    /// Convert a number of microseconds to an interval.
    ///
    /// Saturates to the maximum value if secs is more than `u64::MAX`
    /// nanoseconds.
    pub fn from_micros(micros: u64) -> Self {
        Self {
            duration_ns: micros.saturating_mul(1_000),
        }
    }

    /// Convert a number of nanoseconds to an interval.
    pub fn from_nanos(nanos: u64) -> Self {
        Self { duration_ns: nanos }
    }

    /// Get number of seconds in the current interval.
    ///
    /// Truncates towards zero.
    pub fn as_seconds(self) -> u64 {
        self.duration_ns / 1_000_000_000
    }

    /// Get number of milliseconds in the current interval.
    ///
    /// Truncates towards zero.
    pub fn as_millis(self) -> u64 {
        self.duration_ns / 1_000_000
    }

    /// Get number of microseconds in the current interval.
    ///
    /// Truncates towards zero.
    pub fn as_micros(self) -> u64 {
        self.duration_ns / 1_000
    }

    /// Get number of nanoseconds in the current interval.
    pub fn as_nanos(self) -> u64 {
        self.duration_ns
    }
}

impl Sub<Instant> for Instant {
    type Output = Interval;

    fn sub(self, other: Instant) -> Interval {
        Interval {
            duration_ns: self.ns_since_epoch.saturating_sub(other.ns_since_epoch),
        }
    }
}

impl From<Interval> for crate::MetricValue {
    fn from(intvl: Interval) -> Self {
        Self::Unsigned(intvl.as_nanos())
    }
}
