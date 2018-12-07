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
pub struct Point {
    value: Counter,
    time: Counter,
}

impl Point {
    pub fn new(value: usize, time: usize) -> Self {
        let v = Counter::default();
        v.set(value);
        let t = Counter::default();
        t.set(time);
        Self { value: v, time: t }
    }

    pub fn value(&self) -> usize {
        self.value.get()
    }

    pub fn time(&self) -> usize {
        self.time.get()
    }

    pub fn set(&self, value: usize, time: usize) {
        self.value.set(value);
        self.time.set(time);
    }
}
