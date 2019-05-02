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

use datastructures::Counter;

#[derive(Clone)]
pub struct Ratelimiter {
    available: Counter<u64>,
    capacity: Counter<u64>,
    quantum: Counter<u64>,
    tick: Counter<u64>,
    next: Counter<u64>,
}

const SECOND: u64 = 1_000_000_000;

impl Ratelimiter {
    pub fn new(capacity: u64, quantum: u64, rate: u64) -> Self {
        Self {
            available: Counter::default(),
            capacity: Counter::new(capacity),
            quantum: Counter::new(quantum),
            tick: Counter::new(SECOND / (rate / quantum)),
            next: Counter::new(time::precise_time_ns()),
        }
    }

    pub fn tick(&self) {
        let now = time::precise_time_ns();
        let next = self.next.get();
        if now >= next {
            self.next.increment(self.tick.get());
            self.available.increment(self.quantum.get());
            if self.available.get() > self.capacity.get() {
                self.available.set(self.capacity.get());
            }
        }
    }

    pub fn try_wait(&self) -> Result<(), ()> {
        self.tick();
        self.available.try_decrement(1)
    }

    pub fn wait(&self) {
        // TODO: this can be rewritten as a while loop
        loop {
            self.tick();
            if self.available.try_decrement(1).is_ok() {
                break;
            }
        }
    }
}
