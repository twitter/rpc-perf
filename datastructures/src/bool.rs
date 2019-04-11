// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::wrapper::Wrapper;

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
/// A simple atomic bool which can be shared across threads with many writers
pub struct Bool {
    inner: Wrapper<AtomicBool>,
}

impl Bool {
    /// Create a new bool
    pub fn new(value: bool) -> Self {
        Self {
            inner: Wrapper::new(AtomicBool::new(value)),
        }
    }

    /// Return the value stored in the `Bool`
    pub fn get(&self) -> bool {
        unsafe { (*self.inner.get()).load(Ordering::SeqCst) }
    }

    /// Store a new value in the `Bool`
    pub fn set(&self, value: bool) {
        unsafe {
            (*self.inner.get()).store(value, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time;

    #[test]
    fn flip() {
        let flag = Bool::new(true);
        assert_eq!(flag.get(), true);
        flag.set(false);
        assert_eq!(flag.get(), false);
        flag.set(true);
        assert_eq!(flag.get(), true);
    }

    #[test]
    fn threaded_access() {
        let flag = Bool::new(true);

        let mut threads = Vec::new();

        for _ in 0..2 {
            let flag = flag.clone();
            threads.push(thread::spawn(move || {
                while flag.get() {
                    thread::sleep(time::Duration::from_millis(1));
                }
            }));
        }

        thread::sleep(time::Duration::from_millis(100));

        flag.set(false);

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        assert_eq!(flag.get(), false);
    }
}
