// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::time::Duration;

pub mod bucket;
pub mod latched;
pub mod moving;

pub use self::latched::Latched as LatchedHistogram;
pub use self::moving::Moving as MovingHistogram;

/// A set of common functions for all `Histogram` types
pub trait Histogram {
    /// Clear all samples from histogram
    fn clear(&self);
    /// Return the number of samples seen with the nominal value
    fn count(&self, value: usize) -> usize;
    /// Decrement the number of samples for the nominal value by count
    fn decr(&self, value: usize, count: usize);
    /// Increment the number of samples for the nominal value by count
    fn incr(&self, value: usize, count: usize);
    /// Return the maximum value that can be stored
    fn max(&self) -> usize;
    /// Return the minimum value that can be stored
    fn min(&self) -> usize;
    /// Calculate the percentile (0.0-1.0)
    fn percentile(&self, percentile: f64) -> Option<usize>;
    /// Return the precision in significant figures
    fn precision(&self) -> usize;
    /// Return the number of samples that were below the minimum storable value
    fn too_low(&self) -> usize;
    /// Return the number of samples that were above the maximum storable value
    fn too_high(&self) -> usize;
    /// Return the total number of samples recorded
    fn samples(&self) -> usize;
    /// Return the sum of all the samples
    fn sum(&self) -> Option<usize>;
    /// Return the mean of all samples
    fn mean(&self) -> Option<usize>;
    /// Return the standard deviation of all samples
    fn std_dev(&self) -> Option<usize>;
    /// Return the mode of all samples
    fn mode(&self) -> Option<usize>;
    /// Return the count of samples in the bucket with the most samples
    fn highest_count(&self) -> usize;
    /// Return the number of buckets in the histogram
    fn buckets(&self) -> usize;
}

pub struct Builder {
    min: usize,
    max: usize,
    precision: usize,
    window: Option<Duration>,
}

impl Builder {
    pub fn new(min: usize, max: usize, precision: usize, window: Option<Duration>) -> Self {
        Self {
            min,
            max,
            precision,
            window,
        }
    }

    pub fn build(&self) -> Box<Histogram> {
        if let Some(window) = self.window {
            Box::new(self::MovingHistogram::new(self.max, self.precision, window))
        } else {
            Box::new(self::LatchedHistogram::new(self.max, self.precision))
        }
    }
}
