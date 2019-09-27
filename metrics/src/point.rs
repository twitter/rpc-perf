// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use datastructures::*;

/// Simple datastructure to track a value at a point in time
pub struct Point {
    value: AtomicU64,
    time: AtomicU64,
}

impl Point {
    /// Create a new point from its `value` and `time`
    pub fn new(value: u64, time: u64) -> Self {
        let value = AtomicU64::new(value);
        let time = AtomicU64::new(time);
        Self { value, time }
    }

    /// Get the `value` for the `Point`
    pub fn value(&self) -> u64 {
        self.value.get()
    }

    /// Get the `time` for the `Point`
    pub fn time(&self) -> u64 {
        self.time.get()
    }

    /// Update the `value` and `time` of the `Point` with new values
    pub fn set(&self, value: u64, time: u64) {
        self.value.set(value);
        self.time.set(time);
    }

    /// Zeros out the value and time
    pub fn reset(&self) {
        self.value.set(0);
        self.time.set(0);
    }
}
