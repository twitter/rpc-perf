// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! A basic implementation of a hashed wheel timer

use logger::*;

use core::fmt::Debug;
use std::collections::{HashMap, HashSet};
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
    /// Create a new timer `Wheel` with a given number of `buckets`. Higher
    /// bucket count reduces collisions and results in more efficient
    /// bookkeeping at the expense of additional memory.
    ///
    /// # Example
    ///
    /// ```
    /// use timer::*;
    ///
    /// let timer = Wheel::<usize>::new(1000);
    /// ```
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

    /// Moves the timer forward by a set number of ticks. Any timers that expire
    /// within the provided number of ticks will be returned in a `Vec<T>`
    ///
    /// # Example
    ///
    /// ```
    /// use timer::*;
    /// use std::time::{Duration, Instant};
    ///
    /// let mut timer = Wheel::new(1000);
    /// timer.add(1, 100);
    ///
    /// let mut last_tick = Instant::now();
    ///
    /// loop {
    ///     // do something here
    ///     let elapsed = Instant::now() - last_tick;
    ///     let ticks = elapsed.subsec_millis();
    ///     let expired = timer.tick(ticks as usize);
    ///     if expired.len() > 0 {
    ///         break;
    ///     }
    /// }
    /// ```
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

    // internal function to advance by a single tick
    fn do_tick(&mut self) -> Vec<T> {
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

    /// Adds a new timer for the given token for a number of ticks in the future
    ///
    /// # Examples
    ///
    /// ```
    /// use timer::*;
    ///
    /// let mut timer = Wheel::new(1000);
    ///
    /// timer.add(1, 0); // will expire on next tick
    /// let expired = timer.tick(1);
    /// assert_eq!(expired.len(), 1);
    /// ```
    pub fn add(&mut self, token: T, ticks: usize) {
        trace!("Add timer for {:?} in {} ticks", token, ticks);
        if self.timers.contains_key(&token) {
            self.cancel(token);
        }
        let bucket = (ticks + self.tick) % self.buckets.len();
        let remaining = ticks / self.buckets.len();
        let timer = Timer {
            token,
            remaining,
            bucket,
        };
        self.timers.insert(token, timer);
        self.buckets[bucket].timers.insert(token);
    }

    /// Return the number of timers registered
    ///
    /// # Examples
    ///
    /// ```
    /// use timer::*;
    ///
    /// let mut timer = Wheel::new(1000);
    ///
    /// assert_eq!(timer.pending(), 0);
    ///
    /// timer.add(1, 1);
    /// assert_eq!(timer.pending(), 1);
    /// ```
    pub fn pending(&self) -> usize {
        self.timers.len()
    }

    /// Cancel a pending timer
    ///
    /// # Examples
    ///
    /// ```
    /// use timer::*;
    ///
    /// let mut timer = Wheel::new(1000);
    ///
    /// timer.add(1, 1);
    /// assert_eq!(timer.pending(), 1);
    ///
    /// timer.cancel(1);
    /// assert_eq!(timer.pending(), 0);
    /// ```
    pub fn cancel(&mut self, token: T) {
        if let Some(timer) = self.timers.remove(&token) {
            self.buckets[timer.bucket].timers.remove(&token);
        }
        self.timers.shrink_to_fit();
    }

    /// Return the number of ticks until the next timeout would occur
    ///
    /// # Examples
    ///
    /// ```
    /// use timer::*;
    ///
    /// let mut timer = Wheel::new(1000);
    ///
    /// timer.add(1, 100);
    /// assert_eq!(timer.next_timeout(), Some(100));
    /// ```
    pub fn next_timeout(&self) -> Option<usize> {
        if self.timers.is_empty() {
            None
        } else {
            let mut remaining = 0;
            loop {
                for offset in 0..self.buckets.len() {
                    let mut tick = self.tick + offset;
                    if tick >= self.buckets.len() {
                        tick -= self.buckets.len();
                    }
                    for timer in &self.buckets[tick].timers {
                        if self.timers[&timer].remaining == remaining {
                            return Some(offset + remaining * self.buckets.len());
                        }
                    }
                }
                remaining += 1;
            }
        }
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
        assert_eq!(wheel.next_timeout(), None);
    }

    #[test]
    fn add() {
        let mut wheel = Wheel::new(1000);
        let _id = wheel.add(0, 0);
        assert_eq!(wheel.pending(), 1);
        assert_eq!(wheel.next_timeout(), Some(0));
        let timers = wheel.tick(1);
        assert_eq!(timers.len(), 1);
        assert_eq!(wheel.pending(), 0);
        assert_eq!(wheel.next_timeout(), None);
    }

    #[test]
    fn cancel() {
        let mut wheel = Wheel::new(1000);
        wheel.add(0, 0);
        assert_eq!(wheel.pending(), 1);
        assert_eq!(wheel.next_timeout(), Some(0));
        wheel.cancel(0);
        assert_eq!(wheel.pending(), 0);
        assert_eq!(wheel.next_timeout(), None);
    }

    #[test]
    fn tick() {
        let mut wheel = Wheel::new(1000);
        for i in 0..1000 {
            wheel.add(i, i);
        }
        assert_eq!(wheel.pending(), 1000);
        for i in 0..1000 {
            assert_eq!(wheel.next_timeout(), Some(0));
            let timers = wheel.tick(1);
            assert_eq!(timers.len(), 1);
            assert_eq!(timers[0], i);
        }
        assert_eq!(wheel.pending(), 0);
        assert_eq!(wheel.next_timeout(), None);
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

    #[test]
    fn next_timeout() {
        let mut wheel = Wheel::new(1000);
        wheel.add(1, 5000);
        assert_eq!(wheel.next_timeout(), Some(5000));
        wheel.add(2, 1000);
        assert_eq!(wheel.next_timeout(), Some(1000));
        wheel.add(3, 1);
        assert_eq!(wheel.next_timeout(), Some(1));
    }
}
