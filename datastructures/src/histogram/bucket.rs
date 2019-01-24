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

use crate::counter::Counter;
use crate::wrapper::RwWrapper;
use logger::*;

#[derive(Clone)]
pub struct OuterBucket {
    inner: Bucket,
    buckets: Vec<Bucket>,
    step: RwWrapper<f64>,
}

impl OuterBucket {
    pub fn new(min: usize, max: usize, buckets: usize) -> Self {
        trace!("outer bucket: {} -> {} with {} buckets", min, max, buckets);
        let inner = Bucket::new(min, max);
        let count = buckets;
        let mut buckets = Vec::with_capacity(count);
        let range = max - min;
        let step = range as f64 / count as f64;

        for i in 0..count {
            let bucket_min = (i as f64 * step) as usize + min;
            let bucket_max = ((i + 1) as f64 * step) as usize + min;
            buckets.push(Bucket::new(bucket_min, bucket_max));
        }

        Self {
            inner,
            buckets,
            step: RwWrapper::new(step),
        }
    }

    pub fn get_bucket(&self, index: usize) -> Option<&Bucket> {
        if index < self.buckets() {
            Some(&self.buckets[index])
        } else {
            None
        }
    }

    pub fn buckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn count(&self) -> usize {
        self.inner.count()
    }

    pub fn count_at(&self, value: usize) -> usize {
        let index = self.index(value);
        if let Some(bucket) = self.buckets.get(index) {
            bucket.count()
        } else {
            0
        }
    }

    pub fn min(&self) -> usize {
        self.inner.min()
    }

    pub fn max(&self) -> usize {
        self.inner.max()
    }

    fn index(&self, value: usize) -> usize {
        if value < self.min() {
            error!(
                "value: {} is below outer bucket: {} -> {}",
                value,
                self.min(),
                self.max()
            );
        }
        if value >= self.max() {
            error!(
                "value: {} is above outer bucket: {} -> {}",
                value,
                self.min(),
                self.max()
            );
        }
        let tmp = ((value - self.min()) as f64 / unsafe { *self.step.get() }).floor() as usize;
        if value < self.buckets[tmp].min() {
            tmp - 1
        } else if value >= self.buckets[tmp].max() {
            tmp + 1
        } else {
            tmp
        }
    }

    pub fn incr(&self, value: usize, count: usize) {
        if value < self.min() {
            error!(
                "value: {} is below outer bucket: {} -> {}",
                value,
                self.min(),
                self.max()
            );
        }
        if value >= self.max() {
            error!(
                "value: {} is above outer bucket: {} -> {}",
                value,
                self.min(),
                self.max()
            );
        }
        debug_assert!(value >= self.min());
        debug_assert!(value < self.max());
        if let Some(bucket) = self.buckets.get(self.index(value)) {
            if value < bucket.min() {
                error!(
                    "value: {} is below bucket: {} -> {}",
                    value,
                    bucket.min(),
                    bucket.max()
                );
                error!("outer bucket: {} -> {}", self.min(), self.max());
                error!(
                    "step: {} buckets: {}",
                    unsafe { *self.step.get() },
                    self.buckets.len()
                );
            }
            if value >= bucket.max() {
                error!(
                    "value: {} is above bucket: {} -> {}",
                    value,
                    bucket.min(),
                    bucket.max()
                );
                error!("outer bucket: {} -> {}", self.min(), self.max());
                error!(
                    "step: {} buckets: {}",
                    unsafe { *self.step.get() },
                    self.buckets.len()
                );
            }
            debug_assert!(value >= bucket.min());
            debug_assert!(value < bucket.max());
            bucket.incr(count);
            self.inner.incr(count);
        } else {
            error!(
                "index out of bounds! value: {} outer bucket: {} -> {}",
                value,
                self.min(),
                self.max()
            );
        }
    }

    pub fn decr(&self, value: usize, count: usize) {
        if let Some(bucket) = self.buckets.get(self.index(value)) {
            bucket.decr(count);
            self.inner.decr(count);
        }
    }

    pub fn clear(&self) {
        self.inner.clear();
        for bucket in &self.buckets {
            bucket.clear();
        }
    }

    pub fn percentile(&self, percentile: f64) -> usize {
        let need = (percentile * self.count() as f64).ceil() as usize;
        let mut have = 0;
        for bucket in &self.buckets {
            have += bucket.count();
            if have >= need {
                return (bucket.min() + bucket.max()) / 2;
            }
        }
        self.max()
    }
}

#[derive(Clone)]
pub struct Bucket {
    count: Counter,
    min: Counter,
    max: Counter,
}

impl Bucket {
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            count: Counter::default(),
            min: Counter::new(min),
            max: Counter::new(max),
        }
    }

    pub fn incr(&self, count: usize) {
        self.count.incr(count)
    }

    pub fn decr(&self, count: usize) {
        self.count.decr(count)
    }

    pub fn clear(&self) {
        self.count.clear();
    }

    pub fn count(&self) -> usize {
        self.count.get()
    }

    pub fn min(&self) -> usize {
        self.min.get()
    }

    pub fn max(&self) -> usize {
        self.max.get()
    }

    pub fn width(&self) -> usize {
        self.max() - self.min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn incr_decr() {
        let bucket = Bucket::new(1, 2);
        assert_eq!(bucket.min(), 1);
        assert_eq!(bucket.max(), 2);
        assert_eq!(bucket.count(), 0);
        bucket.incr(1);
        assert_eq!(bucket.count(), 1);
        bucket.decr(1);
        assert_eq!(bucket.count(), 0);
    }

    //#[test]
    fn threaded_access() {
        let bucket = Bucket::new(1, 2);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    bucket.incr(1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        assert_eq!(bucket.count(), 2_000_000);
    }

    #[test]
    fn outer_bucket() {
        let ob = OuterBucket::new(0, 10, 10);
        assert_eq!(ob.buckets(), 10);
        assert_eq!(ob.min(), 0);
        assert_eq!(ob.max(), 10);

        let mut min = ob.min();
        for bucket in &ob.buckets {
            assert_eq!(bucket.min(), min);
            assert_eq!(bucket.max(), min + 1);
            min += 1;
        }

        assert_eq!(ob.index(0), 0);
        assert_eq!(ob.index(1), 1);
        assert_eq!(ob.index(9), 9);

        for i in 0..10 {
            ob.incr(i, 1);
            assert_eq!(ob.count(), i + 1);
        }
        ob.clear();
        assert_eq!(ob.count(), 0);
    }
}
