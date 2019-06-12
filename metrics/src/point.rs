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

pub struct Point {
    value: Counter<u64>,
    time: Counter<u64>,
}

impl Point {
    pub fn new(value: u64, time: u64) -> Self {
        let value = Counter::new(value);
        let time = Counter::new(time);
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
