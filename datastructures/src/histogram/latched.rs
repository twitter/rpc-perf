// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

// a small and simple histogram

use crate::counter::Counting;
use std::convert::From;

use crate::counter::Counter;
use crate::histogram::bucket::Bucket;
use crate::histogram::Histogram;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers
pub struct Latched<C>
where
    C: Counting,
    u64: From<C>,
{
    max: u64,
    buckets: Vec<Counter<C>>,
    too_high: Counter<u64>,
    exact_max: u64,
    precision: usize,
}

impl<C> Latched<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Create a new `Histogram` which will store values between 0 and max
    /// while retaining the precision of the represented values
    pub fn new(max: u64, precision: usize) -> Self {
        let exact_max = 10_u64.pow(precision as u32);
        let mut histogram = Self {
            max,
            buckets: Vec::new(),
            too_high: Counter::default(),
            exact_max,
            precision,
        };
        for _ in 0..=histogram.get_index(max).unwrap() {
            histogram.buckets.push(Counter::<C>::default());
        }
        histogram
    }

    // Internal function to get the max of the linear range of the histogram
    fn exact_max(&self) -> u64 {
        self.exact_max
    }

    // Internal function to get the index for a given value
    fn get_index(&self, value: u64) -> Result<usize, ()> {
        if value > self.max {
            Err(())
        } else if value <= self.exact_max() {
            Ok(value as usize)
        } else {
            let power = (value as f64).log10().floor() as usize;
            let divisor = 10_u64.pow((power - self.precision) as u32 + 1);
            let base_offset = 10_usize.pow(self.precision as u32);
            let power_offset = (0.9
                * (10_usize.pow(self.precision as u32) * (power - self.precision)) as f64)
                as usize;
            let remainder = value / divisor;
            let shift = 10_usize.pow(self.precision as u32 - 1);
            let index = base_offset + power_offset + remainder as usize - shift;
            Ok(index)
        }
    }

    // Internal function to get the value for a given index
    fn get_min_value(&self, index: usize) -> Result<u64, ()> {
        if index >= self.buckets.len() {
            Err(())
        } else if (index as u64) <= self.exact_max() {
            Ok(index as u64)
        } else if index == self.buckets.len() - 1 {
            Ok(self.max)
        } else {
            let shift = 10_usize.pow(self.precision as u32 - 1);
            let base_offset = 10_usize.pow(self.precision as u32);
            let power = self.precision
                + (index - base_offset) / (9 * 10_usize.pow(self.precision as u32 - 1));
            let power_offset = (0.9
                * (10_usize.pow(self.precision as u32) * (power - self.precision)) as f64)
                as usize;
            let value = (index + shift - base_offset - power_offset) as u64
                * 10_u64.pow((power - self.precision + 1) as u32);
            Ok(value)
        }
    }

    fn get_max_value(&self, index: usize) -> Result<u64, ()> {
        if index == self.buckets.len() - 1 {
            Ok(self.max + 1)
        } else {
            Ok(self.get_min_value(index + 1).unwrap())
        }
    }

    // Internal function to get the bucket at a given index
    fn get_bucket(&self, index: usize) -> Option<Bucket<C>> {
        if let Some(counter) = self.buckets.get(index) {
            let bucket = Bucket::new(
                self.get_min_value(index).unwrap(),
                self.get_max_value(index).unwrap(),
            );
            bucket.increment(counter.get());
            Some(bucket)
        } else {
            None
        }
    }

    fn get_value(&self, index: usize) -> Result<u64, ()> {
        self.get_max_value(index).map(|v| v - 1)
    }
}

pub struct Iter<'a, C>
where
    C: Counting,
    u64: From<C>,
{
    inner: &'a Latched<C>,
    index: usize,
}

impl<'a, C> Iter<'a, C>
where
    C: Counting,
    u64: From<C>,
{
    fn new(inner: &'a Latched<C>) -> Iter<'a, C> {
        Iter { inner, index: 0 }
    }
}

impl<'a, C> Iterator for Iter<'a, C>
where
    C: Counting,
    u64: From<C>,
{
    type Item = Bucket<C>;

    fn next(&mut self) -> Option<Bucket<C>> {
        let bucket = self.inner.get_bucket(self.index);
        self.index += 1;
        bucket
    }
}

impl<'a, C> IntoIterator for &'a Latched<C>
where
    C: Counting,
    u64: From<C>,
{
    type Item = Bucket<C>;
    type IntoIter = Iter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<C> Histogram<C> for Latched<C>
where
    C: Counting,
    u64: From<C>,
{
    fn reset(&self) {
        for bucket in &self.buckets {
            bucket.set(Default::default());
        }
        self.too_high.reset();
    }

    fn count(&self, value: u64) -> u64 {
        if let Ok(index) = self.get_index(value) {
            match self.get_bucket(index) {
                Some(bucket) => u64::from(bucket.count()),
                None => 0,
            }
        } else {
            0
        }
    }

    fn decrement(&self, value: u64, count: C) {
        if let Ok(index) = self.get_index(value) {
            self.buckets[index].decrement(count);
        } else {
            self.too_high.decrement(u64::from(count));
        }
    }

    fn increment(&self, value: u64, count: C) {
        if let Ok(index) = self.get_index(value) {
            self.buckets[index].increment(count);
        } else {
            self.too_high.increment(u64::from(count));
        }
    }

    fn max(&self) -> u64 {
        self.max
    }

    fn percentile(&self, percentile: f64) -> Option<u64> {
        if self.samples() == 0 {
            return None;
        }
        let need = if percentile == 0.0 {
            1
        } else {
            (self.samples() as f64 * percentile).ceil() as u64
        };
        let mut have = 0;
        for (index, counter) in self.buckets.iter().enumerate() {
            have += u64::from(counter.get());
            if have >= need {
                return Some(self.get_value(index).unwrap());
            }
        }
        Some(self.max())
    }

    fn precision(&self) -> usize {
        self.precision
    }

    fn too_high(&self) -> u64 {
        self.too_high.get()
    }

    fn samples(&self) -> u64 {
        let mut total = self.too_high();
        for bucket in &self.buckets {
            total += u64::from(bucket.get());
        }
        total
    }

    fn sum(&self) -> Option<u64> {
        if self.samples() == 0 {
            None
        } else {
            let mut sum = 0;
            for bucket in self.into_iter() {
                sum += u64::from(bucket.count()) * bucket.nominal();
            }
            Some(sum)
        }
    }

    fn mean(&self) -> Option<f64> {
        if self.samples() == 0 {
            None
        } else {
            Some(self.sum().unwrap_or(0) as f64 / self.samples() as f64)
        }
    }

    fn std_dev(&self) -> Option<f64> {
        if self.samples() == 0 {
            None
        } else {
            let mean = self.mean().unwrap();
            let mut sum = 0.0;
            for bucket in self.into_iter() {
                sum +=
                    (bucket.nominal() as f64 - mean).powf(2.0) * (u64::from(bucket.count()) as f64);
            }
            Some((sum / self.samples() as f64).powf(0.5))
        }
    }

    fn mode(&self) -> Option<u64> {
        if self.samples() == 0 {
            None
        } else {
            let mut mode = 0;
            let mut count = 0;
            for bucket in self.into_iter() {
                if bucket.weighted_count() > count {
                    count = bucket.weighted_count();
                    mode = u64::from(bucket.count());
                }
            }
            Some(mode)
        }
    }

    fn highest_count(&self) -> Option<u64> {
        if self.samples() == 0 {
            None
        } else {
            let mut highest_count = 0;
            for bucket in self.into_iter() {
                if bucket.weighted_count() > highest_count {
                    highest_count = bucket.weighted_count();
                }
            }
            Some(highest_count)
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
    use std::u64;

    #[test]
    fn bucketing_1() {
        let histogram = Latched::<u64>::new(100, 1);
        assert_eq!(histogram.get_bucket(0).unwrap().min(), 0);
        assert_eq!(histogram.get_bucket(0).unwrap().max(), 1);
        assert_eq!(histogram.get_bucket(0).unwrap().nominal(), 0);
        assert_eq!(histogram.get_bucket(1).unwrap().min(), 1);
        assert_eq!(histogram.get_bucket(1).unwrap().max(), 2);
        assert_eq!(histogram.get_bucket(1).unwrap().nominal(), 1);
        assert_eq!(histogram.get_bucket(2).unwrap().min(), 2);
        assert_eq!(histogram.get_bucket(2).unwrap().max(), 3);
        assert_eq!(histogram.get_bucket(2).unwrap().nominal(), 2);
        assert_eq!(histogram.get_bucket(3).unwrap().min(), 3);
        assert_eq!(histogram.get_bucket(3).unwrap().max(), 4);
        assert_eq!(histogram.get_bucket(3).unwrap().nominal(), 3);
        assert_eq!(histogram.get_bucket(4).unwrap().min(), 4);
        assert_eq!(histogram.get_bucket(4).unwrap().max(), 5);
        assert_eq!(histogram.get_bucket(4).unwrap().nominal(), 4);
        assert_eq!(histogram.get_bucket(5).unwrap().min(), 5);
        assert_eq!(histogram.get_bucket(5).unwrap().max(), 6);
        assert_eq!(histogram.get_bucket(5).unwrap().nominal(), 5);
        assert_eq!(histogram.get_bucket(6).unwrap().min(), 6);
        assert_eq!(histogram.get_bucket(6).unwrap().max(), 7);
        assert_eq!(histogram.get_bucket(6).unwrap().nominal(), 6);
        assert_eq!(histogram.get_bucket(7).unwrap().min(), 7);
        assert_eq!(histogram.get_bucket(7).unwrap().max(), 8);
        assert_eq!(histogram.get_bucket(7).unwrap().nominal(), 7);
        assert_eq!(histogram.get_bucket(8).unwrap().min(), 8);
        assert_eq!(histogram.get_bucket(8).unwrap().max(), 9);
        assert_eq!(histogram.get_bucket(8).unwrap().nominal(), 8);
        assert_eq!(histogram.get_bucket(9).unwrap().min(), 9);
        assert_eq!(histogram.get_bucket(9).unwrap().max(), 10);
        assert_eq!(histogram.get_bucket(9).unwrap().nominal(), 9);
        assert_eq!(histogram.get_bucket(10).unwrap().min(), 10);
        assert_eq!(histogram.get_bucket(10).unwrap().max(), 20);
        assert_eq!(histogram.get_bucket(10).unwrap().nominal(), 19);
    }

    #[test]
    fn get_index_1() {
        let histogram = Latched::<u64>::new(1000000, 1);
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
        assert_eq!(histogram.get_index(1000000), Ok(55));
        assert_eq!(histogram.get_index(1000001), Err(()));
    }

    #[test]
    fn get_index_2() {
        let histogram = Latched::<u64>::new(1000000, 2);
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
        assert_eq!(histogram.get_index(1_000_000), Ok(460));
        assert_eq!(histogram.get_index(1_000_001), Err(()));
    }
    #[test]
    fn get_index_3() {
        let histogram = Latched::<u64>::new(1000000, 3);
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
        assert_eq!(histogram.get_index(1_000_000), Ok(3700));
        assert_eq!(histogram.get_index(1_000_001), Err(()));
    }
    #[test]
    fn get_value_1() {
        let histogram = Latched::<u64>::new(1000000, 1);
        assert_eq!(histogram.get_value(0), Ok(0));
        assert_eq!(histogram.get_value(9), Ok(9));
        assert_eq!(histogram.get_value(10), Ok(19));
        assert_eq!(histogram.get_value(11), Ok(29));
        assert_eq!(histogram.get_value(18), Ok(99));
        assert_eq!(histogram.get_value(19), Ok(199));
    }
    #[test]
    fn get_value_2() {
        let histogram = Latched::<u64>::new(1000000, 2);
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
        let histogram = Latched::<u64>::new(1000000, 3);
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
        let histogram = Latched::<u64>::new(10, 1);
        assert_eq!(histogram.max(), 10);
        assert_eq!(histogram.count(1), 0);
        assert_eq!(histogram.samples(), 0);
        histogram.increment(1, 1);
        assert_eq!(histogram.count(0), 0);
        assert_eq!(histogram.count(1), 1);
        assert_eq!(histogram.count(2), 0);
        assert_eq!(histogram.samples(), 1);
        histogram.decrement(1, 1);
        assert_eq!(histogram.count(0), 0);
        assert_eq!(histogram.count(1), 0);
        assert_eq!(histogram.count(2), 0);
        assert_eq!(histogram.samples(), 0);
    }

    #[test]
    // test clearing the data
    fn clear() {
        let histogram = Latched::<u64>::new(10, 1);
        for i in 0..12 {
            histogram.increment(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 12);
        assert_eq!(histogram.too_high(), 1);
        histogram.reset();
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    // behavior when decrementing past 0
    fn bucket_underflow() {
        let histogram = Latched::<u64>::new(10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.decrement(1, 1);
        assert_eq!(histogram.count(1), u64::MAX);
    }

    #[test]
    // behavior when incrementing past `usize::MAX`
    fn bucket_overflow() {
        let histogram = Latched::<u64>::new(10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.increment(1, u64::MAX);
        assert_eq!(histogram.count(1), u64::MAX);
        histogram.increment(1, 1);
        assert_eq!(histogram.count(1), 0);
    }

    #[test]
    // validate that threaded access yields correct results
    fn threaded_access() {
        let histogram = Latched::<u64>::new(10, 1);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let histogram = histogram.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    histogram.increment(1, 1);
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
        let histogram = Latched::<u64>::new(101, 3);

        for i in 1..101 {
            histogram.increment(i, 1);
        }

        assert_eq!(histogram.percentile(0.0).unwrap(), 1);
        assert_eq!(histogram.percentile(0.01).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 25);
        assert_eq!(histogram.percentile(0.5).unwrap(), 50);
        assert_eq!(histogram.percentile(0.75).unwrap(), 75);
        assert_eq!(histogram.percentile(0.90).unwrap(), 90);
        assert_eq!(histogram.percentile(0.99).unwrap(), 99);
        assert_eq!(histogram.percentile(1.0).unwrap(), 100);
    }

    #[test]
    // test percentiles for a histogram which includes approximate buckets
    fn percentiles_approx() {
        let histogram = Latched::<u64>::new(100, 1);

        for i in 0..101 {
            histogram.increment(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 101);
        assert_eq!(histogram.percentile(0.01).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 29);
        assert_eq!(histogram.percentile(0.5).unwrap(), 59);
        assert_eq!(histogram.percentile(0.75).unwrap(), 79);
        assert_eq!(histogram.percentile(0.90).unwrap(), 99);
        assert_eq!(histogram.percentile(0.99).unwrap(), 99);
        assert_eq!(histogram.percentile(1.0).unwrap(), 100);
    }

    #[test]
    fn too_high() {
        let histogram = Latched::<u64>::new(100, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(102, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_high(), 1);
        histogram.decrement(102, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    fn incr_min() {
        let histogram = Latched::<u64>::new(100, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(1, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    fn incr_max() {
        let histogram = Latched::<u64>::new(100, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(100, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(101, 1);
        assert_eq!(histogram.samples(), 2);
        assert_eq!(histogram.too_high(), 1);
    }

    #[test]
    fn incr_max_large() {
        let max = 80_000_000_000;
        let histogram = Latched::<u64>::new(max, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(max - 1, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_high(), 0);
        histogram.increment(max + 1, 1);
        assert_eq!(histogram.samples(), 2);
        assert_eq!(histogram.too_high(), 1);
    }

    #[test]
    fn mean() {
        let histogram = Latched::<u64>::new(100, 3);
        assert_eq!(histogram.mean(), None);
        assert_eq!(histogram.samples(), 0);
        for i in 0..101 {
            histogram.increment(i, 1);
        }
        assert_eq!(histogram.mean().map(|v| v.round() as usize), Some(50));
        assert_eq!(histogram.samples(), 101);
        histogram.reset();
        histogram.increment(25, 100);
        assert_eq!(histogram.mean(), Some(25.0));
    }

    #[test]
    fn std_dev() {
        let histogram = Latched::<u64>::new(100, 3);
        assert_eq!(histogram.std_dev(), None);
        assert_eq!(histogram.samples(), 0);
        for i in 0..101 {
            histogram.increment(i, 1);
        }
        assert_eq!(histogram.std_dev().map(|v| v.round() as usize), Some(29));
        assert_eq!(histogram.samples(), 101);
        histogram.reset();
        histogram.increment(25, 100);
        assert_eq!(histogram.std_dev(), Some(0.0));
    }
}
