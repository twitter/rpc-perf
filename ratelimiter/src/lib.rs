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

use datastructures::*;

pub struct Ratelimiter {
    available: AtomicU64,
    capacity: AtomicU64,
    quantum: AtomicU64,
    tick: AtomicU64,
    next: AtomicU64,
}

const SECOND: u64 = 1_000_000_000;

impl Ratelimiter {
    pub fn new(capacity: u64, quantum: u64, rate: u64) -> Self {
        Self {
            available: AtomicU64::default(),
            capacity: AtomicU64::new(capacity),
            quantum: AtomicU64::new(quantum),
            tick: AtomicU64::new(SECOND / (rate / quantum)),
            next: AtomicU64::new(time::precise_time_ns()),
        }
    }

    pub fn tick(&self) {
        let now = time::precise_time_ns();
        let next = self.next.get();
        if now >= next {
            self.next.add(self.tick.get());
            self.available.add(self.quantum.get());
            if self.available.get() > self.capacity.get() {
                self.available.set(self.capacity.get());
            }
        }
    }

    pub fn try_wait(&self) -> Result<(), ()> {
        self.tick();
        if self.available.get() > 0 {
            self.available.saturating_sub(1);
            Ok(())
        } else {
            Err(())
        }
        // self.available.try_decrement(1)
    }

    pub fn wait(&self) {
        // TODO: this can be rewritten as a while loop
        loop {
            if self.try_wait().is_ok() {
                break;
            }
        }
    }
}
