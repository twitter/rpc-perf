// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

// a small and simple histogram

use crate::histogram::bucket::Bucket;
use crate::histogram::Histogram;
use crate::counter::Counter;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers
pub struct Simple {
    max: usize,
    buckets: Vec<Counter>,
    too_high: Counter,
    exact_max: Counter,
    precision: Counter,
}

impl Simple {
    /// Create a new `Histogram` which will store values between 0 and max
    /// while retaining the precision of the represented values
    pub fn new(max: usize, precision: usize) -> Self {
        let exact_max = 10_usize.pow(precision as u32); 

        let total_buckets = if exact_max >= max {
            max + 1
        } else {
            exact_max + (((max as f64).log10() as usize) - precision) * 9 * 10_usize.pow(precision as u32 - 1)
        };

        let mut buckets = Vec::new();
        for _ in 0..total_buckets {
            buckets.push(Counter::default());
        }

        Self {
            max,
            buckets,
            too_high: Counter::default(),
            exact_max: Counter::new(exact_max),
            precision: Counter::new(precision),
        }
    }

    // Internal function to get the max of the linear range of the histogram
    fn exact_max(&self) -> usize {
        self.exact_max.get()
    }

    // Internal function to get the index for a given value
    fn get_index(&self, value: usize) -> Result<usize, ()> {
        if value >= self.max {
            Err(())
        } else if value < self.exact_max() {
            Ok(value)
        } else {
            let power = (value as f64).log10().floor() as usize;
            let precision = self.precision.get();
            let divisor = 10_usize.pow((power - precision) as u32 + 1);
            let base_offset = 10_usize.pow(precision as u32);
            let power_offset = (0.9 * (10_usize.pow(precision as u32) * (power - precision)) as f64) as usize;
            let remainder = value / divisor;
            let shift = 10_usize.pow(precision as u32 - 1);
            let index = base_offset + power_offset + remainder - shift;
            Ok(index)
        }
    }

    // Internal function to get the value for a given index
    fn get_value(&self, index: usize) -> Result<usize, ()> {
        if index >= self.buckets.len() {
            Err(())
        } else if index < self.exact_max() {
            Ok(index)
        } else if index == self.buckets.len() - 1 {
            Ok(self.max() - 1)
        } else {
            let index = index + 1;
            let precision = self.precision.get();
            let shift = 10_usize.pow(precision as u32 - 1);
            let base_offset = 10_usize.pow(precision as u32);
            let power = precision + (index - base_offset) / (9 * 10_usize.pow(precision as u32 - 1));
            let power_offset = (0.9 * (10_usize.pow(precision as u32) * (power - precision)) as f64) as usize;
            let value = (index + shift - base_offset - power_offset) * 10_usize.pow((power - precision + 1) as u32);
            Ok(value as usize - 1)
        }
    }

    // Internal function to get the bucket at a given index
    fn get_bucket(&self, index: usize) -> Option<Bucket> {
        if let Some(counter) = self.buckets.get(index) {

            let count = counter.get();

            let min = if index < self.exact_max() {
                if index == 0 {
                    0
                } else {
                    index - 1
                }
            } else {
                self.get_value(index - 1).unwrap()
            };
            let max = if index == self.buckets.len() - 1 {
                self.max()
            } else {
                self.get_value(index).unwrap()
            };
            if min == max {
                println!("bucket: {} for value: {} has min: {} and max: {}", index, self.get_value(index).unwrap(), min, max);
            }
            let bucket = Bucket::new(min, max);
            bucket.incr(count);
            Some(bucket)
        } else {
            None
        }
    }
}

pub struct Iter<'a> {
    inner: &'a Simple,
    index: usize,
}

impl<'a> Iter<'a> {
    fn new(inner: &'a Simple) -> Iter<'a> {
        Iter {
            inner,
            index: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Bucket;

    fn next(&mut self) -> Option<Bucket> {
        let bucket = self.inner.get_bucket(self.index);
        self.index += 1;
        bucket
    }
}

impl<'a> IntoIterator for &'a Simple {
    type Item = Bucket;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl Histogram for Simple {
    fn clear(&self) {
        for bucket in &self.buckets {
            bucket.clear();
        }
        self.too_high.clear();
    }

    fn count(&self, value: usize) -> usize {
        if let Ok(index) = self.get_index(value) {
            match self.get_bucket(index) {
                Some(bucket) => bucket.count(),
                None => 0,
            }
        } else {
            0
        }
    }

    fn decr(&self, value: usize, count: usize) {
        if value >= self.max() {
            self.too_high.decr(count);
        } else if let Ok(index) = self.get_index(value) {
            if let Some(bucket) = self.buckets.get(index) {
                bucket.decr(count);
            } else {
                self.too_high.decr(count);
            }
        } else {
            self.too_high.decr(count);
        }
    }

    fn incr(&self, value: usize, count: usize) {
        if value >= self.max() {
            self.too_high.incr(count);
        } else if let Ok(index) = self.get_index(value) {
            if let Some(bucket) = self.buckets.get(index) {
                bucket.incr(count);
            } else {
                self.too_high.incr(count);
            }
        } else {
            self.too_high.incr(count);
        }
    }

    fn max(&self) -> usize {
        self.max
    }

    fn min(&self) -> usize {
        0
    }

    fn percentile(&self, percentile: f64) -> Option<usize> {
        let samples = self.samples();
        if samples == 0 {
            return None;
        }
        let need = (samples as f64 * percentile).ceil() as usize;
        let mut have = self.too_low();
        if have >= need {
            return Some(self.min());
        } else {
            for (index, bucket) in self.buckets.iter().enumerate() {
                if have + bucket.get() >= need {
                    if let Ok(percentile) = self.get_value(index) {
                        return Some(percentile)
                    } else {
                        return None
                    }
                } else {
                    have += bucket.get();
                }
            }
        }
        Some(self.max())
    }

    fn precision(&self) -> usize {
        self.precision.get()
    }

    fn too_low(&self) -> usize {
        0
    }

    fn too_high(&self) -> usize {
        self.too_high.get()
    }

    fn samples(&self) -> usize {
        let mut total = self.too_low() + self.too_high();
        for bucket in &self.buckets {
            total += bucket.get();
        }
        total
    }

    fn sum(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mut sum = 0;
            for (index, counter) in self.buckets.iter().enumerate() {
                sum += (self.get_value(index).unwrap_or(0)) * counter.get();
            }
            Some(sum)
        }
    }

    fn mean(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let sum = self.sum().unwrap_or(0);
            Some((sum as f64 / self.samples() as f64).ceil() as usize)
        }
    }

    fn std_dev(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mean = self.mean().unwrap();
            let mut sum = 0;
            for (index, counter) in self.buckets.iter().enumerate() {
                sum += (self.get_value(index).unwrap_or(0) as i64 - mean as i64).pow(2) as usize * counter.get();
            }
            Some((sum as f64 / self.samples() as f64).powf(0.5).round() as usize)
        }
    }

    fn mode(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mut mode = 0;
            let mut count = 0;
            for (index, counter) in self.buckets.iter().enumerate() {
                let width = if index < self.exact_max() {
                    1
                } else {
                    self.get_value(index).unwrap_or(1) - self.get_value(index - 1).unwrap_or(0)
                };
                let magnitude = counter.get() / width;
                if magnitude > count {
                    count = magnitude;
                    mode = counter.get();
                }
            }
            Some(mode)
        }
    }

    fn highest_count(&self) -> usize {
        if self.samples() == 0 {
            0
        } else {
            let mut count = 0;
            for (index, counter) in self.buckets.iter().enumerate() {
                let width = if index < self.exact_max() {
                    1
                } else {
                    self.get_value(index).unwrap_or(1) - self.get_value(index - 1).unwrap_or(0)
                };
                let magnitude = counter.get() / width;
                if magnitude > count {
                    count = magnitude;
                }
            }
            count
        }
    }

    fn buckets(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::usize;

    #[test]
    fn get_index_1() {
        let histogram = Simple::new(1000000, 1);
        assert_eq!(histogram.get_index(0), Ok(0));
        assert_eq!(histogram.get_index(9), Ok(9));
        assert_eq!(histogram.get_index(10), Ok(10));
        assert_eq!(histogram.get_index(99), Ok(18));
        assert_eq!(histogram.get_index(100), Ok(19));
        assert_eq!(histogram.get_index(999), Ok(27));
        assert_eq!(histogram.get_index(1000), Ok(28));
        assert_eq!(histogram.get_index(9999), Ok(36));
        assert_eq!(histogram.get_index(10000), Ok(37));
        assert_eq!(histogram.get_index(99999), Ok(45));
        assert_eq!(histogram.get_index(100000), Ok(46));
        assert_eq!(histogram.get_index(999999), Ok(54));
        assert_eq!(histogram.get_index(1000000), Err(()));
    }

    #[test]
    fn get_index_2() {
        let histogram = Simple::new(1000000, 2);
        assert_eq!(histogram.get_index(0), Ok(0));
        assert_eq!(histogram.get_index(99), Ok(99));
        assert_eq!(histogram.get_index(100), Ok(100));
        assert_eq!(histogram.get_index(999), Ok(189));
        assert_eq!(histogram.get_index(1_000), Ok(190));
        assert_eq!(histogram.get_index(9_999), Ok(279));
        assert_eq!(histogram.get_index(10_000), Ok(280));
        assert_eq!(histogram.get_index(99_999), Ok(369));
        assert_eq!(histogram.get_index(100_000), Ok(370));
        assert_eq!(histogram.get_index(999_999), Ok(459));
        assert_eq!(histogram.get_index(1_000_000), Err(()));
    }
    #[test]
    fn get_index_3() {
        let histogram = Simple::new(1000000, 3);
        assert_eq!(histogram.get_index(0), Ok(0));
        assert_eq!(histogram.get_index(99), Ok(99));
        assert_eq!(histogram.get_index(100), Ok(100));
        assert_eq!(histogram.get_index(999), Ok(999));
        assert_eq!(histogram.get_index(1_000), Ok(1000));
        assert_eq!(histogram.get_index(9_999), Ok(1899));
        assert_eq!(histogram.get_index(10_000), Ok(1900));
        assert_eq!(histogram.get_index(99_999), Ok(2799));
        assert_eq!(histogram.get_index(100_000), Ok(2800));
        assert_eq!(histogram.get_index(999_999), Ok(3699));
        assert_eq!(histogram.get_index(1_000_000), Err(()));
    }
    #[test]
    fn get_value_1() {
        let histogram = Simple::new(1000000, 1);
        assert_eq!(histogram.get_value(0), Ok(0));
        assert_eq!(histogram.get_value(9), Ok(9));
        assert_eq!(histogram.get_value(10), Ok(19));
        assert_eq!(histogram.get_value(11), Ok(29));
        assert_eq!(histogram.get_value(18), Ok(99));
        assert_eq!(histogram.get_value(19), Ok(199));
    }
    #[test]
    fn get_value_2() {
        let histogram = Simple::new(1000000, 2);
        assert_eq!(histogram.get_value(0), Ok(0));
        assert_eq!(histogram.get_value(99), Ok(99));
        assert_eq!(histogram.get_value(100), Ok(109));
        assert_eq!(histogram.get_value(189), Ok(999));
        assert_eq!(histogram.get_value(190), Ok(1099));
        assert_eq!(histogram.get_value(279), Ok(9999));
        assert_eq!(histogram.get_value(280), Ok(10999));
        assert_eq!(histogram.get_value(369), Ok(99999));
        assert_eq!(histogram.get_value(0), Ok(0));
    }
    #[test]
    fn get_value_3() {
        let histogram = Simple::new(1000000, 3);
        assert_eq!(histogram.get_value(0), Ok(0));
        assert_eq!(histogram.get_value(99), Ok(99));
        assert_eq!(histogram.get_value(100), Ok(100));
        assert_eq!(histogram.get_value(999), Ok(999));
        assert_eq!(histogram.get_value(1000), Ok(1009));
        assert_eq!(histogram.get_value(1899), Ok(9999));
        assert_eq!(histogram.get_value(1900), Ok(10099));
        assert_eq!(histogram.get_value(2799), Ok(99999));
        assert_eq!(histogram.get_value(0), Ok(0));
    }
    #[test]
    // increment and decrement
    fn incr_decr() {
        let histogram = Simple::new(10, 1);
        assert_eq!(histogram.min(), 0);
        assert_eq!(histogram.max(), 10);
        assert_eq!(histogram.count(1), 0);
        assert_eq!(histogram.samples(), 0);
        histogram.incr(1, 1);
        assert_eq!(histogram.count(0), 0);
        assert_eq!(histogram.count(1), 1);
        assert_eq!(histogram.count(2), 0);
        assert_eq!(histogram.samples(), 1);
        histogram.decr(1, 1);
        assert_eq!(histogram.count(0), 0);
        assert_eq!(histogram.count(1), 0);
        assert_eq!(histogram.count(2), 0);
        assert_eq!(histogram.samples(), 0);
    }

    #[test]
    // test clearing the data
    fn clear() {
        let histogram = Simple::new(10, 1);
        for i in 0..11 {
            histogram.incr(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 11);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 1);
        histogram.clear();
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    // behavior when decrementing past 0
    fn bucket_underflow() {
        let histogram = Simple::new(10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.decr(1, 1);
        assert_eq!(histogram.count(1), usize::MAX);
    }

    #[test]
    // behavior when incrementing past `usize::MAX`
    fn bucket_overflow() {
        let histogram = Simple::new(10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.incr(1, usize::MAX);
        assert_eq!(histogram.count(1), usize::MAX);
        histogram.incr(1, 1);
        assert_eq!(histogram.count(1), 0);
    }

    #[test]
    // validate that threaded access yields correct results
    fn threaded_access() {
        let histogram = Simple::new(10, 1);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let histogram = histogram.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    histogram.incr(1, 1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        assert_eq!(histogram.count(1), 2_000_000);
    }

    #[test]
    // test percentiles for an exact-only histogram
    fn percentiles_exact() {
        let histogram = Simple::new(101, 3);

        for i in 1..101 {
            histogram.incr(i, 1);
        }

        assert_eq!(histogram.percentile(0.01).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 25);
        assert_eq!(histogram.percentile(0.5).unwrap(), 50);
        assert_eq!(histogram.percentile(0.75).unwrap(), 75);
        assert_eq!(histogram.percentile(0.90).unwrap(), 90);
        assert_eq!(histogram.percentile(0.99).unwrap(), 99);
    }

    #[test]
    // test percentiles for a histogram which includes approximate buckets
    fn percentiles_approx() {
        let histogram = Simple::new(101, 1);

        for i in 0..101 {
            histogram.incr(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 101);
        assert_eq!(histogram.percentile(0.01).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 29);
        assert_eq!(histogram.percentile(0.5).unwrap(), 59);
        assert_eq!(histogram.percentile(0.75).unwrap(), 79);
        assert_eq!(histogram.percentile(0.90).unwrap(), 100);
        assert_eq!(histogram.percentile(0.99).unwrap(), 100);
    }

    #[test]
    fn too_high() {
        let histogram = Simple::new(101, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(102, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 1);
        histogram.decr(102, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    fn incr_min() {
        let histogram = Simple::new(101, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(1, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    fn incr_max() {
        let histogram = Simple::new(101, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(99, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(100, 1);
        assert_eq!(histogram.samples(), 2);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 1);
    }

    #[test]
    fn mean() {
        let histogram = Simple::new(101, 3);
        assert_eq!(histogram.mean(), None);
        assert_eq!(histogram.samples(), 0);
        for i in 0..101 {
            histogram.incr(i, 1);
        }
        assert_eq!(histogram.mean(), Some(50));
        assert_eq!(histogram.samples(), 101);
        histogram.clear();
        histogram.incr(25, 100);
        assert_eq!(histogram.mean(), Some(25));
    }

    #[test]
    fn std_dev() {
        let histogram = Simple::new(101, 3);
        assert_eq!(histogram.std_dev(), None);
        assert_eq!(histogram.samples(), 0);
        for i in 0..101 {
            histogram.incr(i, 1);
        }
        assert_eq!(histogram.std_dev(), Some(29));
        assert_eq!(histogram.samples(), 101);
        histogram.clear();
        histogram.incr(25, 100);
        assert_eq!(histogram.std_dev(), Some(0));
    }
}
