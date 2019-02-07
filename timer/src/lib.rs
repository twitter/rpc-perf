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

use core::fmt::Debug;
use logger::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

pub struct Wheel<T> {
    tick: usize,
    buckets: Vec<Bucket<T>>,
    timers: HashMap<T, Timer<T>>,
}

impl<T> Wheel<T>
where
    T: Copy + Clone + Eq + Hash + Debug,
{
    pub fn new(buckets: usize) -> Self {
        let mut wheel = Self {
            tick: 0,
            buckets: Vec::with_capacity(buckets),
            timers: HashMap::new(),
        };
        for _ in 0..buckets {
            wheel.buckets.push(Bucket::new());
        }
        wheel
    }

    pub fn tick(&mut self, ticks: usize) -> Vec<T> {
        let mut timers = Vec::new();
        for _ in 0..ticks {
            timers.extend(self.do_tick());
        }
        if !timers.is_empty() {
            debug!("timeouts: {}", timers.len());
        }
        timers
    }

    pub fn do_tick(&mut self) -> Vec<T> {
        let mut expired = Vec::with_capacity(self.buckets[self.tick].timers.len());
        let mut remaining = HashSet::new();
        for token in &self.buckets[self.tick].timers {
            if self.timers[&token].remaining == 0 {
                expired.push(*token);
                self.timers.remove(&token);
            } else {
                remaining.insert(*token);
                self.timers.get_mut(token).unwrap().remaining -= 1;
            }
        }
        self.buckets[self.tick].timers = remaining;
        if self.tick == (self.buckets.len() - 1) {
            self.tick = 0;
        } else {
            self.tick += 1;
        }
        expired
    }

    pub fn add(&mut self, token: T, ticks: usize) {
        trace!("Add timer for {:?} in {} ticks", token, ticks);
        if self.timers.contains_key(&token) {
            self.cancel(token);
        }
        let bucket = (ticks + self.tick) % self.buckets.len();
        let remaining = ticks / self.buckets.len();
        let timer = Timer { token, remaining, bucket };
        self.timers.insert(token, timer);
        self.buckets[bucket].timers.insert(token);
    }

    pub fn pending(&self) -> usize {
        self.timers.len()
    }

    pub fn cancel(&mut self, token: T) {
        if let Some(timer) = self.timers.remove(&token) {
            self.buckets[timer.bucket].timers.remove(&token);
        }
        self.timers.shrink_to_fit();
    }
}

pub struct Bucket<T> {
    timers: HashSet<T>,
}

impl<T> Bucket<T>
where
    T: Copy + Clone + Eq + Hash + Debug,
{
    pub fn new() -> Self {
        Self {
            timers: HashSet::new(),
        }
    }
}

impl<T> Default for Bucket<T>
where
    T: Copy + Clone + Eq + Hash + Debug,
{
    fn default() -> Bucket<T> {
        Bucket::new()
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Timer<T> {
    bucket: usize,
    remaining: usize,
    token: T,
}

impl<T> Timer<T>
where
    T: Copy + Clone + Eq + Hash + Debug,
{
    pub fn token(&self) -> T {
        self.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let mut wheel = Wheel::<usize>::new(1000);
        assert!(wheel.tick(1000).is_empty());
    }

    #[test]
    fn add() {
        let mut wheel = Wheel::new(1000);
        let _id = wheel.add(0, 0);
        assert_eq!(wheel.pending(), 1);
        let timers = wheel.tick(1);
        assert_eq!(timers.len(), 1);
        assert_eq!(wheel.pending(), 0);
    }

    #[test]
    fn cancel() {
        let mut wheel = Wheel::new(1000);
        wheel.add(0, 0);
        assert_eq!(wheel.pending(), 1);
        wheel.cancel(0);
        assert_eq!(wheel.pending(), 0);
    }

    #[test]
    fn tick() {
        let mut wheel = Wheel::new(1000);
        for i in 0..1000 {
            wheel.add(i, i);
        }
        assert_eq!(wheel.pending(), 1000);
        for i in 0..1000 {
            let timers = wheel.tick(1);
            assert_eq!(timers.len(), 1);
            assert_eq!(timers[0], i);
        }
        assert_eq!(wheel.pending(), 0);
    }

    #[test]
    fn wrap() {
        let mut wheel = Wheel::new(1000);
        for i in 0..2000 {
            wheel.add(i, i);
        }
        assert_eq!(wheel.pending(), 2000);
        for _ in 0..1000 {
            let timers = wheel.tick(1);
            assert_eq!(timers.len(), 1);
        }
        assert_eq!(wheel.pending(), 1000);
        for _ in 0..1000 {
            let timers = wheel.tick(1);
            assert_eq!(timers.len(), 1);
        }
        assert_eq!(wheel.pending(), 0);
    }
}
