// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::counter::Counter;
use crate::counter::Counting;

#[derive(Clone)]
pub struct Bucket<C>
where
    C: Counting,
    u64: From<C>,
{
    count: Counter<C>,
    min: Counter<u64>,
    max: Counter<u64>,
}

impl<C> Bucket<C>
where
    C: Counting,
    u64: From<C>,
{
    pub fn new(min: u64, max: u64) -> Self {
        Self {
            count: Counter::<C>::default(),
            min: Counter::new(min),
            max: Counter::new(max),
        }
    }

    pub fn increment(&self, count: C) {
        self.count.increment(count)
    }

    pub fn decrement(&self, count: C) {
        self.count.decrement(count)
    }

    pub fn reset(&self) {
        self.count.reset();
    }

    pub fn count(&self) -> C {
        self.count.get()
    }

    pub fn min(&self) -> u64 {
        self.min.get()
    }

    pub fn max(&self) -> u64 {
        self.max.get()
    }

    pub fn nominal(&self) -> u64 {
        self.max.get() - 1
    }

    pub fn width(&self) -> u64 {
        self.max() - self.min()
    }

    pub fn weighted_count(&self) -> u64 {
        u64::from(self.count()) / self.width()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn incr_decr() {
        let bucket = Bucket::<u64>::new(1, 2);
        assert_eq!(bucket.min(), 1);
        assert_eq!(bucket.max(), 2);
        assert_eq!(bucket.count(), 0);
        bucket.increment(1);
        assert_eq!(bucket.count(), 1);
        bucket.decrement(1);
        assert_eq!(bucket.count(), 0);
    }

    #[test]
    fn threaded_access() {
        let bucket = Bucket::<u64>::new(1, 2);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    bucket.increment(1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        assert_eq!(bucket.count(), 2_000_000);
    }
}
