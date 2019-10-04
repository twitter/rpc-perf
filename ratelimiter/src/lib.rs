// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! This library provides a thread safe token bucket ratelimitier

use datastructures::*;

/// A token bucket ratelimiter
pub struct Ratelimiter {
    available: AtomicU64,
    capacity: AtomicU64,
    quantum: AtomicU64,
    tick: AtomicU64,
    next: AtomicU64,
}

const SECOND: u64 = 1_000_000_000;

/// A token bucket ratelimiter
impl Ratelimiter {
    /// Create a new token bucket `Ratelimiter` which can hold up to `capacity`
    /// tokens. `quantum` tokens will be added to the bucket at `rate` times
    /// per second. The token bucket initially starts without any tokens, this
    /// ensures the rate does not start high initially.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimiter::*;
    ///
    /// // ratelimit to 1/s with no burst capacity
    /// let ratelimiter = Ratelimiter::new(1, 1, 1);
    ///
    /// // ratelimit to 100/s with bursts up to 10
    /// let ratelimiter = Ratelimiter::new(10, 1, 100);
    /// ```
    pub fn new(capacity: u64, quantum: u64, rate: u64) -> Self {
        Self {
            available: AtomicU64::default(),
            capacity: AtomicU64::new(capacity),
            quantum: AtomicU64::new(quantum),
            tick: AtomicU64::new(SECOND / (rate / quantum)),
            next: AtomicU64::new(time::precise_time_ns()),
        }
    }

    // internal function to move the time forward
    fn tick(&self) {
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

    /// Non-blocking wait for one token, returns an `Ok` if a token was
    /// successfully acquired, returns an `Err` if it would block.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimiter::*;
    ///
    /// let ratelimiter = Ratelimiter::new(1, 1, 100);
    /// for i in 0..100 {
    ///     // do some work here
    ///     while ratelimiter.try_wait().is_err() {
    ///         // do some other work
    ///     }
    /// }
    /// ```
    pub fn try_wait(&self) -> Result<(), ()> {
        self.tick();
        if self.available.get() > 0 {
            self.available.saturating_sub(1);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Blocking wait implemented as a busy loop. Returns only after a token is
    /// successfully acquired
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimiter::*;
    ///
    /// let ratelimiter = Ratelimiter::new(1, 1, 100);
    /// for i in 0..100 {
    ///     // do some work here
    ///     ratelimiter.wait();
    /// }
    /// ```
    pub fn wait(&self) {
        while self.try_wait().is_err() {}
    }
}
