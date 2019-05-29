// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::counter::Counting;
use std::convert::From;
use std::marker::PhantomData;
use std::time::Duration;

pub mod bucket;
pub mod circular;
pub mod latched;
pub mod moving;

pub use self::circular::Circular as CircularHistogram;
pub use self::latched::Latched as LatchedHistogram;
pub use self::moving::Moving as MovingHistogram;

/// A set of common functions for all `Histogram` types
/// Histograms are generic across the internal counter representation to allow
/// for reduced-memory utilization when anticipating low counts
pub trait Histogram<C> {
    /// Reset all counters to zero
    fn reset(&self);
    /// Return the number of samples seen with the nominal value. The count is
    /// converted up to `u64` even if the bucket counters are lower precision.
    fn count(&self, value: u64) -> u64;
    /// Decrement the number of samples for the nominal value by count
    fn decrement(&self, value: u64, count: C);
    /// Increment the number of samples for the nominal value by count
    fn increment(&self, value: u64, count: C);
    /// Return the maximum value that can be stored
    fn max(&self) -> u64;
    /// Calculate the percentile (0.0-1.0)
    fn percentile(&self, percentile: f64) -> Option<u64>;
    /// Return the precision in significant figures
    fn precision(&self) -> usize;
    /// Return the number of samples that were above the maximum storable value
    fn too_high(&self) -> u64;
    /// Return the total number of samples recorded
    fn samples(&self) -> u64;
    /// Return the sum of all the samples
    fn sum(&self) -> Option<u64>;
    /// Return the mean of all samples
    fn mean(&self) -> Option<f64>;
    /// Return the standard deviation of all samples
    fn std_dev(&self) -> Option<f64>;
    /// Return the mode of all samples
    fn mode(&self) -> Option<u64>;
    /// Return the count of samples in the bucket with the most samples
    fn highest_count(&self) -> Option<u64>;
    /// Return the number of buckets in the histogram
    fn buckets(&self) -> usize;
}

/// `Builder` allows for creating types implementing the `Histogram` trait, the
/// exact imlementation will vary with the `Builder`'s configuration
pub struct Builder<C> {
    max: u64,
    precision: usize,
    window: Option<Duration>,
    capacity: Option<u32>,
    _counter: PhantomData<C>,
}

impl<C: 'static> Builder<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Create a new `Builder` with the given configuration
    pub fn new(
        max: u64,
        precision: usize,
        window: Option<Duration>,
        capacity: Option<u32>,
    ) -> Self {
        Self {
            max,
            precision,
            window,
            capacity,
            _counter: PhantomData::<C>,
        }
    }

    /// Builds a structure which implements the `Histogram` trait that satisfies
    /// the `Builder`'s configuration
    pub fn build(&self) -> Box<Histogram<C>> {
        if let Some(window) = self.window {
            if let Some(capacity) = self.capacity {
                Box::new(self::CircularHistogram::<C>::new(
                    self.max,
                    self.precision,
                    window.as_nanos() as u64,
                    capacity,
                ))
            } else {
                Box::new(self::MovingHistogram::<C>::new(
                    self.max,
                    self.precision,
                    window,
                ))
            }
        } else {
            Box::new(self::LatchedHistogram::<C>::new(self.max, self.precision))
        }
    }
}
