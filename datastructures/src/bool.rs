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
