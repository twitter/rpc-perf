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

use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;

/// `Wrapper` can be used to wrap a thread-safe datatype, such as an atomic it
/// provides for interior mutability for multiple writers
pub struct Wrapper<T> {
    value: Arc<UnsafeCell<T>>,
}

impl<T> Wrapper<T> {
    /// Create a new `Wrapper` containing the given value
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(UnsafeCell::new(value)),
        }
    }

    /// Get a mutable pointer to the inner value
    pub fn get(&self) -> *mut T {
        self.value.get()
    }
}

impl<T> Clone for Wrapper<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

// mark the `Wrapper` as Send and Sync
unsafe impl<T> Send for Wrapper<T> {}
unsafe impl<T> Sync for Wrapper<T> {}

/// `RwWrapper` can be used to provide guarded writes in-addition to
/// thread-safe multiple-writer scenarios. This can be used if the underlying
/// datastructure must be locked at runtime for resizing. There is additional
/// overhead for using this type.
pub struct RwWrapper<T> {
    value: Arc<RwLock<Wrapper<T>>>,
}

impl<T> RwWrapper<T> {
    /// Create a new `RwWrapper` containing the given value
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(Wrapper::new(value))),
        }
    }

    /// Get a mutable pointer to the inner value without taking a lock. This
    /// should only be used for thread-safe actions on atomics. This will block
    /// if there is a locked write
    pub fn get(&self) -> *mut T {
        self.value.read().get()
    }

    /// Get a mutable pointer to the inner value by taking a lock. This can
    /// be used when a non-thread-safe action must be taken on the inner type,
    /// such as resizing the inner datastructure. Taking a lock will cause all
    /// `get()` to block until the lock is released
    pub fn lock(&self) -> *mut T {
        self.value.write().get()
    }
}

impl<T> Clone for RwWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

// mark `RwWrapper` as Send and Sync
unsafe impl<T> Send for RwWrapper<T> {}
unsafe impl<T> Sync for RwWrapper<T> {}
