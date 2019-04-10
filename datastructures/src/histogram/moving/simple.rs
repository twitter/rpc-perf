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
use crate::histogram::latched::simple::Iter;
use crate::histogram::{Histogram, LatchedHistogram};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time;

#[derive(Clone)]
/// A thread-safe fixed-size `Histogram` which allows multiple writers and
/// retains samples across a given `Duration`
pub struct Simple {
    data: LatchedHistogram,
    samples: Arc<Mutex<VecDeque<Sample>>>,
    window: Arc<time::Duration>,
}

impl Default for Simple {
    fn default() -> Simple {
        Self::new(1_000_000, 3, time::Duration::new(60, 0))
    }
}

enum Direction {
    Decrement,
    Increment,
}

struct Sample {
    value: usize,
    count: usize,
    time: time::Instant,
    direction: Direction,
}

impl<'a> IntoIterator for &'a Simple {
    type Item = Bucket;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl Simple {
    /// Create a new `MovingHistogram` with the given min, max, precision, and window
    pub fn new(max: usize, precision: usize, window: time::Duration) -> Self {
        Self {
            data: LatchedHistogram::new(max, precision),
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
                    Direction::Decrement => self.data.incr(sample.value, sample.count),
                    Direction::Increment => self.data.decr(sample.value, sample.count),
                }
            } else {
                queue.push_front(sample);
                break;
            }
        }
    }
}

impl Histogram for Simple {
    /// Remove all samples from the datastructure
    fn clear(&self) {
        self.data.clear();
        self.samples.lock().unwrap().truncate(0);
    }

    /// Get total count of entries
    fn samples(&self) -> usize {
        let time = time::Instant::now();
        self.trim(time);
        self.data.samples()
    }

    /// Decrement the bucket that represents value by count
    fn decr(&self, value: usize, count: usize) {
        let time = time::Instant::now();
        self.trim(time);
        self.data.decr(value, count);
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
    fn incr(&self, value: usize, count: usize) {
        let time = time::Instant::now();
        self.trim(time);
        self.data.incr(value, count);
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
    fn percentile(&self, percentile: f64) -> Option<usize> {
        let time = time::Instant::now();
        self.trim(time);
        self.data.percentile(percentile)
    }

    fn count(&self, value: usize) -> usize {
        let time = time::Instant::now();
        self.trim(time);
        self.data.count(value)
    }

    fn too_high(&self) -> usize {
        let time = time::Instant::now();
        self.trim(time);
        self.data.too_high()
    }

    fn too_low(&self) -> usize {
        let time = time::Instant::now();
        self.trim(time);
        self.data.too_low()
    }

    fn max(&self) -> usize {
        self.data.max()
    }

    fn min(&self) -> usize {
        self.data.min()
    }

    fn precision(&self) -> usize {
        self.data.precision()
    }

    fn sum(&self) -> Option<usize> {
        self.data.sum()
    }

    fn mean(&self) -> Option<usize> {
        self.data.mean()
    }

    fn std_dev(&self) -> Option<usize> {
        self.data.std_dev()
    }

    fn mode(&self) -> Option<usize> {
        self.data.mode()
    }

    fn highest_count(&self) -> usize {
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
        let h = Simple::default();
        assert_eq!(h.samples(), 0);
    }

    #[test]
    fn rolloff() {
        let h = Simple::new(10, 3, time::Duration::new(2, 0));
        assert_eq!(h.samples(), 0);
        h.incr(1, 1);
        assert_eq!(h.samples(), 1);
        thread::sleep(time::Duration::new(1, 0));
        assert_eq!(h.samples(), 1);
        thread::sleep(time::Duration::new(2, 0));
        assert_eq!(h.samples(), 0);
    }

    #[test]
    fn threaded_access() {
        let histogram = Simple::new(10, 3, time::Duration::new(10, 0));

        let mut threads = Vec::new();

        for _ in 0..2 {
            let histogram = histogram.clone();
            threads.push(thread::spawn(move || {
                for i in 0..100_000 {
                    histogram.incr(i, 1);
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
