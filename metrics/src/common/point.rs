// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use datastructures::*;

/// Simple datastructure to track a value at a point in time
pub struct Point {
    value: Atomic<u64>,
    time: Atomic<u64>,
}

impl Point {
    /// Create a new point from its `value` and `time`
    pub fn new(value: u64, time: u64) -> Self {
        let value = Atomic::<u64>::new(value);
        let time = Atomic::<u64>::new(time);
        Self { value, time }
    }

    /// Get the `value` for the `Point`
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Get the `time` for the `Point`
    pub fn time(&self) -> u64 {
        self.time.load(Ordering::Relaxed)
    }

    /// Update the `value` and `time` of the `Point` with new values
    pub fn set(&self, value: u64, time: u64) {
        self.value.store(value, Ordering::Relaxed);
        self.time.store(time, Ordering::Relaxed);
    }

    /// Zeros out the value and time
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
        self.time.store(0, Ordering::Relaxed);
    }
}
