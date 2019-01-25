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
    available: Counter,
    capacity: Counter,
    quantum: Counter,
    tick: Counter,
    next: Counter,
}

const SECOND: usize = 1_000_000_000;

impl Ratelimiter {
    pub fn new(capacity: usize, quantum: usize, rate: usize) -> Self {
        Self {
            available: Counter::default(),
            capacity: Counter::new(capacity),
            quantum: Counter::new(quantum),
            tick: Counter::new(SECOND / (rate / quantum)),
            next: Counter::new(time::precise_time_ns() as usize),
        }
    }

    pub fn tick(&self) {
        let now = time::precise_time_ns() as usize;
        let next = self.next.get();
        if now >= next {
            self.next.incr(self.tick.get());
            self.available.incr(self.quantum.get());
            if self.available.get() > self.capacity.get() {
                self.available.set(self.capacity.get());
            }
        }
    }

    pub fn try_wait(&self) -> Result<(), ()> {
        self.tick();
        self.available.saturating_sub(1)
    }

    pub fn wait(&self) {
        // TODO: this can be rewritten as a while loop
        loop {
            self.tick();
            if self.available.saturating_sub(1).is_ok() {
                break;
            }
        }
    }
}
