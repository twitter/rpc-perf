// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use super::{DDSketchError, DDSketchErrorKind};
use crate::histogram::{Unsigned};

use atomics::*;

/// An atomic DDSketch.
pub struct AtomicDDSketch<T = u64>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    T: Default + Copy,
    u64: From<T>,
{
    buckets: Vec<Atomic<T>>,

    gamma: f64,
    /// Number of linear-sized buckets before we switch to log sized buckets.
    ///
    /// This saves space since below a certain point log-sized buckets have a
    /// width of less than 1 which is useless since we are storign integers.
    cutoff: usize,

    count: Atomic<u64>,
    limit: u64,
    min: Atomic<u64>,
    max: Atomic<u64>,
}

impl<T> AtomicDDSketch<T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    T: Default + Copy,
    u64: From<T>,
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
            // Don't need an overflow bucket if the entire range is covered.
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

            count: Default::default(),
            limit,
            min: Atomic::<u64>::new(std::u64::MAX),
            max: Default::default(),
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

    /// The number of buckets in the sketch.
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Get the bucket-index of a value.
    fn index_of(&self, value: u64) -> usize {
        match value {
            x if x > self.limit => self.buckets.len() - 1,
            x if x < self.cutoff as u64 => x as usize,
            x => (x as f64).log(self.gamma).ceil() as usize + 1,
        }
    }

    /// Increment the bucket holding `value` by `count`.
    pub fn increment(&self, value: u64, count: T) {
        self.max.store_max(value, Ordering::Relaxed);
        self.min.store_min(value, Ordering::Relaxed);
        self.count.saturating_add(count.into(), Ordering::Relaxed);

        self.buckets[self.index_of(value)].saturating_add(count, Ordering::Relaxed);
    }

    /// Clear all buckets within the sketch.
    ///
    /// Note that clearing while other threads are inserting
    /// into the sketch is likely to leave it in a somewhat
    /// inconsistent state.
    pub fn clear(&self) {
        self.min.store(std::u64::MAX, Ordering::Relaxed);
        self.max.store(std::u64::MAX, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);

        for bucket in self.buckets.iter() {
            bucket.store(Default::default(), Ordering::Relaxed);
        }
    }

    /// Total count of samples in the sketch.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// The number of samples that were over the limit and are too
    /// high to store in any given bucket.
    pub fn too_high(&self) -> u64 {
        if self.limit == std::u64::MAX {
            return 0;
        }

        self.buckets.last().unwrap().load(Ordering::Relaxed).into()
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
    ///
    /// # Note
    /// If the sketches are being updated concurrently then rank
    /// calculation may end up being innaccurate if the sketch is
    /// updated during the calculation.
    pub fn quantile(&self, q: f64) -> u64 {
        if q.is_nan() {
            return 0;
        }

        let min = self.min.load(Ordering::Relaxed);
        let max = self.max.load(Ordering::Relaxed);

        if q < 0.0 {
            return min;
        }
        if q >= 1.0 {
            return max;
        }

        let rank = (q * self.count() as f64) as u64;
        let index = self
            .buckets
            .iter()
            .scan(0u64, |total, count| {
                *total += u64::from(count.load(Ordering::Relaxed));
                Some(*total)
            })
            .enumerate()
            .skip_while(|&(_, count)| count <= rank)
            .map(|(i, _)| i)
            .next();

        let index = match index {
            Some(idx) if idx < self.cutoff => idx,
            Some(idx) if idx == self.buckets.len() - 1 => return max,
            Some(idx) => idx - 1,
            None => return max,
        };

        ((self.gamma.powi(index as i32) / (0.5 * (self.gamma + 1.0))).round() as u64)
            .min(max)
            .max(min)
    }

    /// Merge two different sketches.
    ///
    /// This function will return an error if the number of buckets is
    /// different between the two sketches.
    ///
    /// # Note
    /// If the sketches are being updated concurrently then merging
    /// can end up being incorrect due to observing different states
    /// of the sketch.
    pub fn merge(&self, other: &Self) -> Result<(), DDSketchError> {
        if self.num_buckets() != other.num_buckets() {
            return Err(DDSketchError::new(DDSketchErrorKind::Unmergeable));
        }

        self.buckets
            .iter()
            .zip(other.buckets.iter())
            .for_each(|(x, y)| {
                x.saturating_add(y.load(Ordering::Relaxed), Ordering::Relaxed);
            });

        self.count.saturating_add(other.count(), Ordering::Relaxed);
        self.min
            .store_min(other.min.load(Ordering::Relaxed), Ordering::Relaxed);
        self.max
            .store_max(other.max.load(Ordering::Relaxed), Ordering::Relaxed);

        Ok(())
    }

    /// Get the approximate rank of `value` within the sketch.
    ///
    /// For any given distribution this may be arbitrarily inaccurate depending
    /// on what fraction of the values in the sketch are mapped the same bucket.
    ///
    /// # Note
    /// If the sketches are being updated concurrently then quantile
    /// calculation may end up being innaccurate if the sketch is
    /// updated during the quantile calculation.
    pub fn rank(&self, value: u64) -> u64 {
        let index = self.index_of(value);

        self.buckets[..index]
            .iter()
            .map(|x| u64::from(x.load(Ordering::Relaxed)))
            .sum()
    }
}
