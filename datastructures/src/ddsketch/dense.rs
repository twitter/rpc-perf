// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use super::{DDSketchError, DDSketchErrorKind};
use crate::counter::Saturating;

use std::convert::TryFrom;
use std::ops::AddAssign;

/// A non-atomic DDSketch.
///
/// This implementation should be preferred over `AtomicDDSketch`
/// when concurrent insertion into the sketch is not needed.
pub struct DenseDDSketch<T = u64> {
    buckets: Vec<T>,

    gamma: f64,
    /// Number of linear-sized buckets before we switch to log sized buckets.
    ///
    /// This saves space since below a certain point log-sized buckets have a
    /// width of less than 1 which is useless since we are storign integers.
    cutoff: usize,

    min: u64,
    max: u64,
    count: u64,
    limit: u64,
}

// Utility functions that don't require knowing anything about T
impl<T> DenseDDSketch<T>
where
    T: Copy,
{
    /// Get the bucket-index of a value.
    fn index_of(&self, value: u64) -> usize {
        match value {
            x if x > self.limit => self.buckets.len() - 1,
            x if x < self.cutoff as u64 => x as usize,
            x => (x as f64).log(self.gamma).ceil() as usize + 1,
        }
    }

    /// Total count of samples in the sketch.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// The number of buckets in the sketch.
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Maximum value present in the sketch
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Minimum value present in the sketch
    pub fn min(&self) -> u64 {
        self.min
    }

    /// Maximum value that the sketch is sized to store.
    ///
    /// Note that there is another bucket to catch values
    /// greater than this so they don't get lost, but there
    /// are no precision guarantees.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Indicates whether the sketch has no values within it.
    ///
    /// This is the same as checking if `count() == 0`.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

impl<T> DenseDDSketch<T>
where
    T: Saturating + Default + PartialEq + Copy + Into<u64>,
{
    /// Create a sketch that can store values up to `limit` with
    /// a relative precision of `alpha`.
    ///
    /// Returns an error if `alpha` is not in the range `(0, 1)` or if
    /// *log<sub>(1 + alpha)/(1 - alpha)</sub>(limit)* is greater
    /// than `std::i32::MAX`.
    pub fn with_limit(limit: u64, alpha: f64) -> Result<Self, DDSketchError> {
        if alpha <= 0.0 || alpha >= 1.0 {
            return Err(DDSketchError::new(DDSketchErrorKind::InvalidAlpha));
        }

        // Here's how the formula below works.
        //
        // First, to make the formulas shorter, define
        //    γ = (1 + α) / (1 - α)
        // This is the ratio between the maximum and minumum value
        // within a bucket.
        //
        // So we want to figure out the number of buckets that we store
        // within the sketch. The naive way to do this is to use
        //     log_γ(limit) + 1
        // buckets. (log_γ(limit) buckets between 1 and limit and 1 extra
        // for zero.) However, this is wasteful since there will be a bunch
        // of buckets near 1 that will have width smaller than 1 and so
        // would be permanently empty. Instead, we'd like to have an exact
        // histogram up to a certain number, then switch over to the log-sized
        // histogram.
        //
        // The best point to do this is once the log-sized have a size of 1.
        // This happens at index
        //     log_γ (1 / (γ - 1)) = -log_γ(γ - 1)
        // which, when rounded up, becomes
        //     ceil(-log_γ(γ - 1))
        //
        // Putting it all together, the total number of buckets (assuming
        // that limit is larger than the cutoff between buckets) is
        //     ceil(-log_γ(γ - 1)) + ceil(log_γ(limit) - log_γ(γ - 1)) + 1
        //
        // This is mostly what the below code does with some exceptions
        // for handling limits below the cutoff.

        let gamma = (1.0 + alpha) / (1.0 - alpha);
        let log_gamma_m1 = (gamma - 1.0).log(gamma);
        let log_limit = (limit as f64).log(gamma).ceil();

        let cutoff = (-log_gamma_m1).ceil();
        let rest = ((limit as f64).log(gamma) - log_gamma_m1).ceil();

        // Note: We keep two extra buckets
        //  - one for 0
        //  - one for values above the limit
        let mut num_buckets = if log_limit <= cutoff {
            log_limit as usize + 2
        } else {
            cutoff as usize + rest as usize + 2
        };

        if limit == std::u64::MAX {
            // Don't need an overflow bucket if the entire range
            // is covered.
            num_buckets -= 1;
        }

        // Need to keep the maximum exponent less than std::i32::MAX
        // since powi takes an i32.
        if log_limit > std::i32::MAX as f64 {
            return Err(DDSketchError::new(DDSketchErrorKind::TooManyBuckets));
        }

        let mut buckets = Vec::new();
        buckets.resize_with(num_buckets, Default::default);

        Ok(Self {
            buckets,
            gamma,
            cutoff: cutoff as usize + 1,

            count: 0,
            limit,
            min: std::u64::MAX,
            max: std::u64::MIN,
        })
    }

    /// Create a sketch that can store any `u64` with a relative
    /// precision of `alpha`.
    ///
    /// Returns an error if `alpha` is not in the range `(0, 1)` or if
    /// *log<sub>(1 + alpha)/(1 - alpha)</sub>(std::u64::MAX)* is greater
    /// than `std::i32::MAX`.
    pub fn new(alpha: f64) -> Result<Self, DDSketchError> {
        Self::with_limit(std::u64::MAX, alpha)
    }

    /// Increment the bucket holding `value` by `count`.
    pub fn increment(&mut self, value: u64, count: T) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.count = self.count.saturating_add(count.into());

        let index = self.index_of(value);
        saturating_inc(&mut self.buckets[index], count);
    }

    /// Total count of samples in the sketch.
    pub fn clear(&mut self) {
        self.max = std::u64::MIN;
        self.min = std::u64::MAX;
        self.count = 0;

        for bucket in self.buckets.iter_mut() {
            *bucket = T::default();
        }
    }

    /// Merge two different sketches.
    ///
    /// This function will return an error if the number of buckets is
    /// different between the two sketches.
    pub fn merge(&mut self, other: &Self) -> Result<(), DDSketchError> {
        if self.num_buckets() != other.num_buckets() {
            return Err(DDSketchError::new(DDSketchErrorKind::Unmergeable));
        }

        self.count = self.count.saturating_add(other.count);
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);

        self.buckets
            .iter_mut()
            .zip(other.buckets.iter().copied())
            .for_each(|(x, y)| saturating_inc(x, y));

        Ok(())
    }

    /// Returns the approximate value of the quantile specified from
    /// 0.0 to 1.0.
    ///
    /// Any value returned that is within the range [0, limit] will be
    /// accurate within a relative error of `alpha` provided that no
    /// counters within the sketch have saturated.
    ///
    /// If those two conditions are not met, then no error bounds are
    /// given for the returned quantile.
    pub fn quantile(&self, q: f64) -> u64 {
        if q.is_nan() {
            return 0;
        }

        if q < 0.0 {
            return self.min;
        }
        if q >= 1.0 {
            return self.max;
        }

        let rank = (q * self.count as f64) as u64;
        let index = self
            .buckets
            .iter()
            .scan(0u64, |total: &mut u64, &count| {
                *total += count.into();
                Some(*total)
            })
            .enumerate()
            .skip_while(|&(_, count)| count <= rank)
            .map(|(i, _)| i)
            .next();

        let index = match index {
            Some(idx) if idx < self.cutoff => idx,
            Some(idx) if idx == self.buckets.len() - 1 => return self.max,
            Some(idx) => idx - 1,
            None => return self.max,
        };

        ((self.gamma.powi(index as i32) / (0.5 * (self.gamma + 1.0))).round() as u64)
            .min(self.max)
            .max(self.min)
    }

    /// The number of samples that were over the limit and are too
    /// high to store in any given bucket.
    pub fn too_high(&self) -> u64 {
        if self.limit == std::u64::MAX {
            return 0;
        }

        (*self.buckets.last().unwrap()).into()
    }

    /// Get the approximate rank of `value` within the sketch.
    ///
    /// For any given distribution this may be arbitrarily inaccurate depending
    /// on what fraction of the values in the sketch are mapped the same bucket.
    pub fn rank(&self, value: u64) -> u64 {
        let index = self.index_of(value);

        self.buckets[..index]
            .iter()
            .map(|&x| -> u64 { x.into() })
            .sum()
    }
}

fn saturating_inc<T>(loc: &mut T, val: T)
where
    T: Saturating,
{
    *loc = loc.saturating_add(val);
}

impl<T> Extend<u64> for DenseDDSketch<T>
where
    T: Saturating + AddAssign<T> + Default + PartialEq + Copy + Into<u64> + TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
{
    fn extend<I: IntoIterator<Item = u64>>(&mut self, iter: I) {
        let one = T::try_from(1).expect("1 is not convertable to T");
        for item in iter {
            self.increment(item, one)
        }
    }
}

#[test]
fn test_clear() {
    let mut sketch = DenseDDSketch::<u64>::with_limit(64, 0.5).expect("Failed to create sketch");

    assert!(sketch.is_empty());
    assert_eq!(sketch.min(), std::u64::MAX);
    assert_eq!(sketch.max(), std::u64::MIN);

    sketch.increment(11, 33);
    sketch.increment(888, 34);
    sketch.increment(61, 33);

    assert_eq!(sketch.count(), 100);
    assert_eq!(sketch.max(), 888);
    assert_eq!(sketch.min(), 11);
    assert!(!sketch.is_empty());

    sketch.clear();

    assert!(sketch.is_empty());
    assert_eq!(sketch.count(), 0);
    assert_eq!(sketch.min(), std::u64::MAX);
    assert_eq!(sketch.max(), std::u64::MIN);
}
