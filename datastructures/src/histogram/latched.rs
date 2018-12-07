//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use crate::histogram::bucket::Bucket;
use crate::counter::Counter;
use crate::histogram::bucket::OuterBucket;
use crate::histogram::Histogram;

use std::mem;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers
pub struct Latched {
    min: usize,
    max: usize,
    buckets: Vec<OuterBucket>,
    too_low: Counter,
    too_high: Counter,
    exact_max: Counter,
    exact_power: Counter,
    inner_buckets: Counter,
    precision: Counter,
}

impl Latched {
    /// Create a new `Histogram` which will store values between min and max
    /// (inclusive) while retaining the precision of the represented values
    pub fn new(min: usize, max: usize, precision: usize) -> Self {
        let inner_buckets = 10_usize.pow(precision as u32);

        let exact_power = mem::size_of::<usize>() * 8 - inner_buckets.leading_zeros() as usize;
        let mut exact_max = 2_usize.pow(exact_power as u32);

        let max_power = mem::size_of::<usize>() * 8 - max.leading_zeros() as usize;
        let outer_buckets = max_power.saturating_sub(exact_power);

        if exact_max >= max {
            exact_max = max + 1;
        }
        let exact_buckets = exact_max - min;

        let mut buckets = Vec::with_capacity(outer_buckets + 1);
        buckets.push(OuterBucket::new(min, exact_max, exact_buckets));

        for power in exact_power..max_power {
            let min = 2_usize.pow(power as u32);
            let max = 2_usize.pow(power as u32 + 1);
            buckets.push(OuterBucket::new(min, max, inner_buckets));
        }

        Self {
            min,
            max,
            buckets,
            too_high: Counter::default(),
            too_low: Counter::default(),
            exact_max: Counter::new(exact_max),
            exact_power: Counter::new(exact_power),
            inner_buckets: Counter::new(inner_buckets),
            precision: Counter::new(precision),
        }
    }

    // Internal function to get the max of the linear range of the histogram
    fn exact_max(&self) -> usize {
        self.exact_max.get()
    }

    // Internal function to get the power of the max of the linear range
    fn exact_power(&self) -> usize {
        self.exact_power.get()
    }

    // Internal function to get the bucket at a given index
    fn get_bucket(&self, index: usize) -> Option<&OuterBucket> {
        self.buckets.get(index)
    }

    // Internal function to get the index for a given value
    fn get_index(&self, value: usize) -> Result<usize, ()> {
        if value < self.min() || value >= self.max() {
            Err(())
        } else if value < self.exact_max() {
            Ok(0)
        } else {
            let power = mem::size_of::<usize>() * 8 - value.leading_zeros() as usize;
            let index = power.saturating_sub(self.exact_power());
            debug_assert!(index < self.buckets.len());
            Ok(index)
        }
    }

    // Internal function to get the number of inner buckets per outer bucket
    fn inner_buckets(&self) -> usize {
        self.inner_buckets.get()
    }
}

pub struct Iter<'a> {
    inner: &'a Latched,
    outer_index: usize,
    inner_index: usize,
}

impl<'a> Iter<'a> {
    fn new(inner: &'a Latched) -> Iter<'a> {
        Iter { inner, outer_index: 0, inner_index: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Bucket;

    fn next(&mut self) -> Option<&'a Bucket> {
        if self.inner_index >= self.inner.buckets[self.outer_index].buckets() {
            self.outer_index += 1;
            self.inner_index = 0;
        }

        if self.outer_index >= self.inner.buckets.len() {
            None
        } else {
            self.inner_index += 1;
            self.inner.buckets[self.outer_index].get_bucket(self.inner_index - 1)
        }
    }
}

impl<'a> IntoIterator for &'a Latched {
    type Item = &'a Bucket;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl Histogram for Latched {
    fn clear(&self) {
        for bucket in &self.buckets {
            bucket.clear();
        }
        self.too_low.clear();
        self.too_high.clear();
    }

    fn count(&self, value: usize) -> usize {
        if let Ok(index) = self.get_index(value) {
            match self.get_bucket(index) {
                Some(bucket) => bucket.count_at(value),
                None => 0,
            }
        } else {
            0
        }
    }

    fn decr(&self, value: usize, count: usize) {
        if value < self.min() {
            self.too_low.decr(count);
        } else if value > self.max() {
            self.too_high.decr(count);
        } else if let Ok(index) = self.get_index(value) {
            if let Some(bucket) = self.buckets.get(index) {
                bucket.decr(value, count);
            } else {
                self.too_high.decr(count);
            }
        } else {
            self.too_high.decr(count);
        }
    }

    fn incr(&self, value: usize, count: usize) {
        if value < self.min() {
            self.too_low.incr(count);
        } else if value > self.max() {
            self.too_high.incr(count);
        } else if let Ok(index) = self.get_index(value) {
            if let Some(bucket) = self.buckets.get(index) {
                bucket.incr(value, count);
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
        self.min
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
            for bucket in &self.buckets {
                if have + bucket.count() >= need {
                    let percentile =
                        bucket.percentile((need - have) as f64 / bucket.count() as f64);
                    if percentile > self.max() {
                        return Some(self.max());
                    } else {
                        return Some(percentile);
                    }
                } else {
                    have += bucket.count();
                }
            }
        }
        Some(self.max() - 1)
    }

    fn precision(&self) -> usize {
        self.precision.get()
    }

    fn too_low(&self) -> usize {
        self.too_low.get()
    }

    fn too_high(&self) -> usize {
        self.too_high.get()
    }

    fn samples(&self) -> usize {
        let mut total = self.too_low() + self.too_high();
        for bucket in &self.buckets {
            total += bucket.count();
        }
        total
    }

    fn sum(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mut sum = 0;
            for bucket in self {
                sum += (bucket.max() - 1) * bucket.count();
            }
            Some(sum)
        }
    }

    fn mean(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mut sum = 0;
            for bucket in self {
                sum += (bucket.max() - 1) * bucket.count();
            }
            Some((sum as f64 / self.samples() as f64).ceil() as usize)
        }
        
    }

    fn std_dev(&self) -> Option<usize> {
        if self.samples() == 0 {
            None
        } else {
            let mean = self.mean().unwrap();
            let mut sum = 0;
            for bucket in self {
                sum += ((bucket.max() - 1) as i64 - mean as i64).pow(2) as usize * bucket.count();
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
            for bucket in self {
                let magnitude = bucket.count() / bucket.width();
                if magnitude > count {
                    count = magnitude;
                    mode = bucket.max() - 1;
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
            for bucket in self {
                // let magnitude = bucket.count();
                let magnitude = bucket.count() / bucket.width();
                if magnitude > count {
                    count = magnitude;
                }
            }
            count
        }
    }

    fn buckets(&self) -> usize {
        let mut count = 0;
        for bucket in &self.buckets {
            count += bucket.buckets();
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::usize;

    #[test]
    // increment and decrement
    fn incr_decr() {
        let histogram = Latched::new(1, 10, 1);
        assert_eq!(histogram.min(), 1);
        assert_eq!(histogram.max(), 10);
        assert_eq!(histogram.count(1), 0);
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
        let histogram = Latched::new(1, 10, 1);
        for i in 0..11 {
            histogram.incr(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 11);
        assert_eq!(histogram.too_low(), 1);
        assert_eq!(histogram.too_high(), 1);
        histogram.clear();
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    // behavior when decrementing past 0
    fn bucket_underflow() {
        let histogram = Latched::new(1, 10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.decr(1, 1);
        assert_eq!(histogram.count(1), usize::MAX);
    }

    #[test]
    // behavior when incrementing past `usize::MAX`
    fn bucket_overflow() {
        let histogram = Latched::new(1, 10, 1);
        assert_eq!(histogram.count(1), 0);
        histogram.incr(1, usize::MAX);
        assert_eq!(histogram.count(1), usize::MAX);
        histogram.incr(1, 1);
        assert_eq!(histogram.count(1), 0);
    }

    #[test]
    // validate that threaded access yields correct results
    fn threaded_access() {
        let histogram = Latched::new(1, 10, 1);

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
        let histogram = Latched::new(1, 101, 3);

        for i in 1..101 {
            histogram.incr(i, 1);
        }

        assert_eq!(histogram.percentile(0.0).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 25);
        assert_eq!(histogram.percentile(0.5).unwrap(), 50);
        assert_eq!(histogram.percentile(0.75).unwrap(), 75);
        assert_eq!(histogram.percentile(0.90).unwrap(), 90);
        assert_eq!(histogram.percentile(0.99).unwrap(), 99);
    }

    #[test]
    // test percentiles for a histogram which includes approximate buckets
    fn percentiles_approx() {
        let histogram = Latched::new(1, 101, 1);

        for i in 0..101 {
            histogram.incr(i, 1);
            assert_eq!(histogram.samples(), i + 1);
        }
        assert_eq!(histogram.samples(), 101);
        assert_eq!(histogram.percentile(0.0).unwrap(), 1);
        assert_eq!(histogram.percentile(0.25).unwrap(), 26);
        assert_eq!(histogram.percentile(0.5).unwrap(), 49);
        assert_eq!(histogram.percentile(0.75).unwrap(), 73);
        assert_eq!(histogram.percentile(0.90).unwrap(), 92);
        assert_eq!(histogram.percentile(0.99).unwrap(), 99);
    }

    #[test]
    fn too_low() {
        let histogram = Latched::new(1, 101, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(0, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_low(), 1);
        assert_eq!(histogram.too_high(), 0);
        histogram.decr(0, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
    }

    #[test]
    fn too_high() {
        let histogram = Latched::new(1, 101, 1);
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
        let histogram = Latched::new(1, 101, 1);
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
        let histogram = Latched::new(1, 101, 1);
        assert_eq!(histogram.samples(), 0);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(100, 1);
        assert_eq!(histogram.samples(), 1);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 0);
        histogram.incr(101, 1);
        assert_eq!(histogram.samples(), 2);
        assert_eq!(histogram.too_low(), 0);
        assert_eq!(histogram.too_high(), 1);
    }

    #[test]
    fn mean() {
        let histogram = Latched::new(0, 101, 3);
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
        let histogram = Latched::new(0, 101, 3);
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
