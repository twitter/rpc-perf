// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::counter::Counting;
use std::convert::From;

use crate::histogram::bucket::Bucket;
use crate::histogram::latched::Iter;
use crate::histogram::latched::Latched;
use crate::histogram::Histogram;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers and
/// retains samples across a given `Duration`
pub struct Moving<C>
where
    C: Counting,
    u64: From<C>,
{
    data: Latched<C>,
    samples: Arc<Mutex<VecDeque<Sample<C>>>>,
    window: Arc<time::Duration>,
}

enum Direction {
    Decrement,
    Increment,
}

struct Sample<C>
where
    C: Counting,
    u64: From<C>,
{
    value: u64,
    count: C,
    time: time::Instant,
    direction: Direction,
}

impl<'a, C> IntoIterator for &'a Moving<C>
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

impl<C> Moving<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Create a new `MovingHistogram` with the given min, max, precision, and window
    pub fn new(max: u64, precision: usize, window: time::Duration) -> Self {
        Self {
            data: Latched::<C>::new(max, precision),
            samples: Arc::new(Mutex::new(VecDeque::new())),
            window: Arc::new(window),
        }
    }

    // internal function to expire old samples
    fn trim(&self, time: time::Instant) {
        let mut queue = self.samples.lock().unwrap();
        while let Some(sample) = queue.pop_front() {
            let age = time - sample.time;
            if age > (*self.window) {
                match sample.direction {
                    Direction::Decrement => self.data.increment(sample.value, sample.count),
                    Direction::Increment => self.data.decrement(sample.value, sample.count),
                }
            } else {
                queue.push_front(sample);
                break;
            }
        }
    }
}

impl<C> Histogram<C> for Moving<C>
where
    C: Counting,
    u64: From<C>,
{
    /// Remove all samples from the datastructure
    fn reset(&self) {
        self.data.reset();
        self.samples.lock().unwrap().truncate(0);
    }

    /// Get total count of entries
    fn samples(&self) -> u64 {
        let time = time::Instant::now();
        self.trim(time);
        self.data.samples()
    }

    /// Decrement the bucket that represents value by count
    fn decrement(&self, value: u64, count: C) {
        let time = time::Instant::now();
        self.trim(time);
        self.data.decrement(value, count);
        let sample = Sample {
            value,
            count,
            time,
            direction: Direction::Decrement,
        };
        let mut queue = self.samples.lock().unwrap();
        queue.push_back(sample);
    }

    /// Increment the bucket that represents value by count
    fn increment(&self, value: u64, count: C) {
        let time = time::Instant::now();
        self.trim(time);
        self.data.increment(value, count);
        let sample = Sample {
            value,
            count,
            time,
            direction: Direction::Increment,
        };
        let mut queue = self.samples.lock().unwrap();
        queue.push_back(sample);
    }

    /// Return the value for the given percentile (0.0 - 1.0)
    fn percentile(&self, percentile: f64) -> Option<u64> {
        let time = time::Instant::now();
        self.trim(time);
        self.data.percentile(percentile)
    }

    fn count(&self, value: u64) -> u64 {
        let time = time::Instant::now();
        self.trim(time);
        self.data.count(value)
    }

    fn too_high(&self) -> u64 {
        let time = time::Instant::now();
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
        let h = Moving::<u64>::new(10, 3, time::Duration::new(2, 0));
        assert_eq!(h.samples(), 0);
    }

    #[test]
    fn rolloff() {
        let h = Moving::<u64>::new(10, 3, time::Duration::new(2, 0));
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
        let histogram = Moving::<u64>::new(10, 3, time::Duration::new(10, 0));

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
        assert_eq!(histogram.samples(), 200_000);
        thread::sleep(time::Duration::new(11, 0));
        assert_eq!(histogram.samples(), 0);
    }
}
