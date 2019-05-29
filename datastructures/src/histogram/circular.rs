// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::bool::Bool;
use crate::counter::Counter;
use crate::counter::Counting;
use std::convert::From;

use crate::histogram::bucket::Bucket;
use crate::histogram::latched::Iter;
use crate::histogram::latched::Latched;
use crate::histogram::Histogram;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers and
/// retains up to N samples across a given `Duration`
pub struct Circular<C>
where
    C: Counting,
    u64: From<C>,
{
    data: Latched<C>,
    samples: Vec<Sample<C>>,
    oldest: Counter<u32>,
    newest: Counter<u32>,
    used: Counter<u32>,
    window: Counter<u64>,
}

#[derive(Clone)]
struct Sample<C>
where
    C: Counting,
    u64: From<C>,
{
    value: Counter<u64>,
    count: Counter<C>,
    time: Counter<u64>,
    decrement: Bool,
}

impl<C> Default for Sample<C>
where
    C: Counting,
    u64: From<C>,
{
    fn default() -> Sample<C> {
        Sample {
            value: Default::default(),
            count: Default::default(),
            time: Default::default(),
            decrement: Bool::new(false),
        }
    }
}

impl<'a, C> IntoIterator for &'a Circular<C>
where
    C: Counting,
    u64: From<C>,
{
    type Item = Bucket<C>;
    type IntoIter = Iter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<C> Circular<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Create a new `MovingHistogram` with the given max, precision, window, and capacity
    pub fn new(max: u64, precision: usize, window: u64, capacity: u32) -> Self {
        let mut samples = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            samples.push(Sample::default());
        }
        Self {
            data: Latched::<C>::new(max, precision),
            samples,
            newest: Default::default(),
            oldest: Default::default(),
            used: Default::default(),
            window: Counter::new(window),
        }
    }

    // internal function to expire old samples
    fn trim(&self, time: u64) {
        let expired = time - self.window.get();
        if self.used.get() == 0 {
            return;
        }
        let oldest = self.oldest.get() as usize;
        let newest = self.newest.get() as usize;
        if oldest < newest {
            for i in oldest..newest {
                if self.samples[i].time.get() < expired {
                    self.data
                        .decrement(self.samples[i].value.get(), self.samples[i].count.get());
                    self.oldest.increment(1);
                    if self.oldest.get() as usize >= self.samples.len() {
                        self.oldest.set(0);
                    }
                    self.used.decrement(1);
                } else {
                    return;
                }
            }
        } else {
            for i in oldest..self.samples.len() {
                if self.samples[i].time.get() < expired {
                    self.data
                        .decrement(self.samples[i].value.get(), self.samples[i].count.get());
                    self.oldest.increment(1);
                    if self.oldest.get() as usize >= self.samples.len() {
                        self.oldest.set(0);
                    }
                    self.used.decrement(1);
                } else {
                    return;
                }
            }
            for i in 0..newest {
                if self.samples[i].time.get() < expired {
                    self.data
                        .decrement(self.samples[i].value.get(), self.samples[i].count.get());
                    self.oldest.increment(1);
                    if self.oldest.get() as usize >= self.samples.len() {
                        self.oldest.set(0);
                    }
                    self.used.decrement(1);
                } else {
                    return;
                }
            }
        }
    }
}

impl<C> Histogram<C> for Circular<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Remove all samples from the datastructure
    fn reset(&self) {
        self.data.reset();
        self.used.reset();
        self.oldest.reset();
        self.newest.reset();
    }

    /// Get total count of entries
    fn samples(&self) -> u64 {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.samples()
    }

    /// Decrement the bucket that represents value by count
    fn decrement(&self, value: u64, count: C) {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.decrement(value, count);
        if self.used.get() as usize == self.samples.len() {
            let oldest = self.oldest.get() as usize;
            if self.samples[oldest].decrement.get() {
                self.data.increment(
                    self.samples[oldest].value.get(),
                    self.samples[oldest].count.get(),
                );
            } else {
                self.data.decrement(
                    self.samples[oldest].value.get(),
                    self.samples[oldest].count.get(),
                );
            }
            self.oldest.increment(1);
            if self.oldest.get() as usize >= self.samples.len() {
                self.oldest.set(0);
            }
            self.used.decrement(1);
        }
        let newest = self.newest.get() as usize;
        if newest >= self.samples.len() - 1 {
            self.newest.set(0);
            self.samples[0].count.set(count);
            self.samples[0].value.set(value);
            self.samples[0].time.set(time);
            self.samples[0].decrement.set(true);
        } else {
            self.newest.increment(1);
            self.samples[newest].count.set(count);
            self.samples[newest].value.set(value);
            self.samples[newest].time.set(time);
            self.samples[newest].decrement.set(true);
        }
        self.used.increment(1);
    }

    /// Increment the bucket that represents value by count
    fn increment(&self, value: u64, count: C) {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.increment(value, count);
        if self.used.get() as usize == self.samples.len() {
            let oldest = self.oldest.get() as usize;
            if self.samples[oldest].decrement.get() {
                self.data.increment(
                    self.samples[oldest].value.get(),
                    self.samples[oldest].count.get(),
                );
            } else {
                self.data.decrement(
                    self.samples[oldest].value.get(),
                    self.samples[oldest].count.get(),
                );
            }
            self.oldest.increment(1);
            if self.oldest.get() as usize >= self.samples.len() {
                self.oldest.set(0);
            }
            self.used.decrement(1);
        }
        let newest = self.newest.get() as usize;
        if newest >= self.samples.len() - 1 {
            self.newest.set(0);
            self.samples[0].count.set(count);
            self.samples[0].value.set(value);
            self.samples[0].time.set(time);
            self.samples[0].decrement.set(false);
        } else {
            self.newest.increment(1);
            self.samples[newest].count.set(count);
            self.samples[newest].value.set(value);
            self.samples[newest].time.set(time);
            self.samples[newest].decrement.set(false);
        }
        self.used.increment(1);
    }

    /// Return the value for the given percentile (0.0 - 1.0)
    fn percentile(&self, percentile: f64) -> Option<u64> {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.percentile(percentile)
    }

    fn count(&self, value: u64) -> u64 {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.count(value)
    }

    fn too_high(&self) -> u64 {
        let time = time::precise_time_ns();
        self.trim(time);
        self.data.too_high()
    }

    fn max(&self) -> u64 {
        self.data.max()
    }

    fn precision(&self) -> usize {
        self.data.precision()
    }

    fn sum(&self) -> Option<u64> {
        self.data.sum()
    }

    fn mean(&self) -> Option<f64> {
        self.data.mean()
    }

    fn std_dev(&self) -> Option<f64> {
        self.data.std_dev()
    }

    fn mode(&self) -> Option<u64> {
        self.data.mode()
    }

    fn highest_count(&self) -> Option<u64> {
        self.data.highest_count()
    }

    fn buckets(&self) -> usize {
        self.data.buckets()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[test]
    fn empty() {
        let h = Circular::<u64>::new(10, 3, 2_000_000_000, 1000);
        assert_eq!(h.samples(), 0);
    }

    #[test]
    fn rolloff() {
        let h = Circular::<u64>::new(10, 3, 2_000_000_000, 1000);
        assert_eq!(h.samples(), 0);
        h.increment(1, 1);
        assert_eq!(h.samples(), 1);
        thread::sleep(time::Duration::new(1, 0));
        assert_eq!(h.samples(), 1);
        thread::sleep(time::Duration::new(2, 0));
        assert_eq!(h.samples(), 0);
    }

    #[test]
    fn threaded_access() {
        let histogram = Circular::<u64>::new(10, 3, 10_000_000_000, 2000000);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let histogram = histogram.clone();
            threads.push(thread::spawn(move || {
                for i in 0..100_000 {
                    histogram.increment(i, 1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }
        thread::sleep(time::Duration::new(5, 0));
        assert_eq!(histogram.samples(), 200_000);
        thread::sleep(time::Duration::new(6, 0));
        assert_eq!(histogram.samples(), 0);
    }
}
