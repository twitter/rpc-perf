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

use crate::wrapper::Wrapper;

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
/// A simple atomic counter which can be shared across threads with many writers
pub struct Counter {
    inner: Wrapper<AtomicUsize>,
}

impl Default for Counter {
    fn default() -> Self {
        Counter::new(0)
    }
}

impl Counter {
    /// Create a new zeroed counter
    pub fn new(value: usize) -> Self {
        Self {
            inner: Wrapper::new(AtomicUsize::new(value)),
        }
    }

    /// Clear the counter by reseting the value to zero
    pub fn clear(&self) {
        self.set(0);
    }

    /// Return the count stored in the counter
    pub fn get(&self) -> usize {
        unsafe { (*self.inner.get()).load(Ordering::Relaxed) }
    }

    /// Decrement the counter by count
    pub fn decr(&self, count: usize) {
        unsafe {
            (*self.inner.get()).fetch_sub(count, Ordering::Relaxed);
        }
    }

    /// Increment the counter by count
    pub fn incr(&self, count: usize) {
        unsafe {
            (*self.inner.get()).fetch_add(count, Ordering::Relaxed);
        }
    }

    pub fn set(&self, count: usize) {
        unsafe {
            (*self.inner.get()).store(count, Ordering::SeqCst);
        }
    }

    pub fn saturating_sub(&self, count: usize) -> Result<(), ()> {
        let current = self.get();
        let new = current - count;
        if new > current {
            return Err(());
        }
        let result =
            unsafe { (*self.inner.get()).compare_and_swap(current, new, Ordering::SeqCst) };
        if result == current {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::usize;

    #[test]
    fn incr_decr() {
        let counter = Counter::default();
        assert_eq!(counter.get(), 0);
        counter.incr(1);
        assert_eq!(counter.get(), 1);
        counter.decr(1);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn underflow() {
        let counter = Counter::default();
        assert_eq!(counter.get(), 0);
        counter.decr(1);
        assert_eq!(counter.get(), usize::MAX);
    }

    #[test]
    fn overflow() {
        let counter = Counter::default();
        assert_eq!(counter.get(), 0);
        counter.incr(usize::MAX);
        assert_eq!(counter.get(), usize::MAX);
        counter.incr(1);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn threaded_access() {
        let counter = Counter::default();

        let mut threads = Vec::new();

        for _ in 0..2 {
            let counter = counter.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    counter.incr(1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        assert_eq!(counter.get(), 2_000_000);
    }
}
