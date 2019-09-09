// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use datastructures::*;

pub struct Point {
    value: AtomicU64,
    time: AtomicU64,
}

impl Point {
    pub fn new(value: u64, time: u64) -> Self {
        let value = AtomicU64::new(value);
        let time = AtomicU64::new(time);
        Self { value, time }
    }

    pub fn value(&self) -> u64 {
        self.value.get()
    }

    pub fn time(&self) -> u64 {
        self.time.get()
    }

    pub fn set(&self, value: u64, time: u64) {
        self.value.set(value);
        self.time.set(time);
    }

    pub fn reset(&self) {
        self.value.set(0);
        self.time.set(0);
    }
}
