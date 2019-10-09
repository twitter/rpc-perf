// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#![allow(missing_docs)]

const MAX_PERCENTILE: u32 = 1_000_000_000u32;

/// A percentile. Used to calculate percentiles from a summary.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Percentile {
    // Representation is a fixed-point number from 1 to 1000000000
    // with 1000000000 being 1 and 0 being 0.
    val: u32,
}

macro_rules! p {
    ($val:expr) => {
        Percentile { val: $val }
    };
}

impl Percentile {
    /// Create a new percentile from a value between `0` and `1_000_000_000`.
    ///
    /// If the value is out of range, returne `None`.
    pub fn new(val: u32) -> Option<Self> {
        if val > MAX_PERCENTILE {
            return None;
        }
        Some(Percentile { val })
    }

    /// Create a new percentile from a value between `0` and `1_000_000_000`.
    ///
    /// Given the current limitations of const fn, this function is the
    /// only way to create a percentile in a const context.
    ///
    /// # Safety
    /// This function is unsafe since it allows for an invalid `Percentile`
    /// to be created.
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        Self { val }
    }

    /// Get the inner integer representation of this percentile.
    pub const fn into_inner(self) -> u32 {
        self.val
    }

    /// Convert a float to a `Percentile`.
    ///
    /// For floats outside the range of 0 to 1 this function will clamp
    /// them to 0 or 1 respectively.
    pub fn from_float(val: f64) -> Self {
        Self {
            val: (val.min(1.0).max(0.0) * (MAX_PERCENTILE as f64)) as u32,
        }
    }

    /// Convert this percentile to a float.
    pub fn as_float(self) -> f64 {
        (self.val as f64) / (MAX_PERCENTILE as f64)
    }

    /// The 0th percentile. This corresponds to the minimum value within
    /// the sample.
    pub const fn minimum() -> Self {
        p!(0)
    }

    /// The 100th percentile. This corresponds to the maximum value within
    /// the sample.
    pub const fn maximum() -> Self {
        p!(MAX_PERCENTILE)
    }

    /// The 0.01th percentile. This corresponds to the value that is greater
    /// than 0.01% of the sample.
    pub const fn p001() -> Self {
        p!(Self::maximum().val / 10000)
    }

    /// The 0.1th percentile. This corresponds to the value that is greater
    /// than 0.1% of the sample.
    pub const fn p01() -> Self {
        p!(Self::maximum().val / 1000)
    }

    /// The 1st percentile. This corresponds to the value that is greater
    /// than 1% of the sample.
    pub const fn p1() -> Self {
        p!(Self::maximum().val / 100)
    }

    /// The 5th percentile. This correspnods to the value that is greater
    /// than 5% of the sample.
    pub const fn p5() -> Self {
        p!(Self::maximum().val / 20)
    }

    /// The 10th percentile. This corresponds to the value that is greater
    /// than 10% of the sample.
    pub const fn p10() -> Self {
        p!(Self::maximum().val / 10)
    }

    /// The 25th percentile. This corresponds to the value that is greater
    /// than 25% of the sample.
    pub const fn p25() -> Self {
        p!(Self::maximum().val / 4)
    }

    /// The 50th percentile. This corresponds to the value that is greater
    /// than 50% of the sample.
    ///
    /// Note that this is the median of the sample.
    pub const fn p50() -> Self {
        p!(Self::maximum().val / 2)
    }

    /// The 75th percentile. This corresponds to the value that is greater than
    /// 75% of the sample.
    pub const fn p75() -> Self {
        p!(Self::p25().val * 3)
    }

    /// The 90th percentile. This corresponds to the value that is greater than
    /// 90% of the sample.
    pub const fn p90() -> Self {
        p!(Self::p10().val * 9)
    }

    /// The 95th percentile. This corresponds to the value that is greater than
    /// 95% of the sample.
    pub const fn p95() -> Self {
        p!(Self::maximum().val - Self::p5().val)
    }

    /// The 99th percentile. This corresponds to the value that is greater than
    /// 99% of the sample.
    pub const fn p99() -> Self {
        p!(Self::maximum().val - Self::p1().val)
    }

    /// The 99.9th percentile. This corresponds to the value that is greater
    /// than 99.9% of the sample.
    pub const fn p999() -> Self {
        p!(Self::maximum().val - Self::p01().val)
    }

    /// The 99.99th percentile. This corresponds to the value that is greater
    /// than 99.99% of the sample.
    pub const fn p9999() -> Self {
        p!(Self::maximum().val - Self::p001().val)
    }
}

impl From<Percentile> for f64 {
    fn from(x: Percentile) -> f64 {
        x.as_float()
    }
}

impl From<f64> for Percentile {
    fn from(x: f64) -> Self {
        Self::from_float(x)
    }
}
