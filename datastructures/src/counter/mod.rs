// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::wrapper::Wrapper;
use core::ops::*;

use std::fmt;
use std::sync::atomic::*;

#[derive(Clone)]
/// Counter is a thread-safe counter which uses atomics internally. See each
/// function for its `Ordering`
pub struct Counter<T>
where
    T: Counting,
{
    value: Wrapper<T>,
}

pub trait Counting:
    Copy + Default + Add + Sub + Mul + Div + PartialEq + PartialOrd + From<u8>
{
}
impl Counting for u8 {}
impl Counting for u16 {}
impl Counting for u32 {}
impl Counting for u64 {}

unsafe impl<T: Copy + Send> Sync for Counter<T> where T: Counting {}

impl<T> Default for Counter<T>
where
    T: Counting,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> fmt::Debug for Counter<T>
where
    T: Counting + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Counter").field(&self.get()).finish()
    }
}

impl<T> Counter<T>
where
    T: Counting,
{
    /// Creates a new `Counter` holding a provided initial value
    pub fn new(value: T) -> Counter<T> {
        Counter {
            value: Wrapper::new(value),
        }
    }

    /// Returns the value in the `Counter` using `Ordering::Relaxed`
    pub fn get(&self) -> T {
        unsafe {
            match std::mem::size_of::<T>() {
                1 if std::mem::align_of::<T>() >= 1 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU8)).load(Ordering::Relaxed),
                ),
                2 if std::mem::align_of::<T>() >= 2 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU16)).load(Ordering::Relaxed),
                ),
                4 if std::mem::align_of::<T>() >= 4 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU32)).load(Ordering::Relaxed),
                ),
                8 if std::mem::align_of::<T>() >= 8 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU64)).load(Ordering::Relaxed),
                ),
                _ => panic!("failed to load atomic"),
            }
        }
    }

    /// Stores the value in the `Counter` using `Ordering::SeqCst`
    pub fn set(&self, value: T) {
        unsafe {
            match std::mem::size_of::<T>() {
                1 if std::mem::align_of::<T>() >= 1 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU8))
                        .store(std::mem::transmute_copy(&value), Ordering::SeqCst),
                ),
                2 if std::mem::align_of::<T>() >= 2 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU16))
                        .store(std::mem::transmute_copy(&value), Ordering::SeqCst),
                ),
                4 if std::mem::align_of::<T>() >= 4 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU32))
                        .store(std::mem::transmute_copy(&value), Ordering::SeqCst),
                ),
                8 if std::mem::align_of::<T>() >= 8 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU64))
                        .store(std::mem::transmute_copy(&value), Ordering::SeqCst),
                ),
                _ => panic!("failed to set atomic"),
            }
        }
    }

    /// Adds the value to the `Counter` using `Ordering::Relaxed`
    pub fn increment(&self, value: T) {
        unsafe {
            match std::mem::size_of::<T>() {
                1 if std::mem::align_of::<T>() >= 1 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU8))
                        .fetch_add(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                2 if std::mem::align_of::<T>() >= 2 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU16))
                        .fetch_add(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                4 if std::mem::align_of::<T>() >= 4 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU32))
                        .fetch_add(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                8 if std::mem::align_of::<T>() >= 8 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU64))
                        .fetch_add(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                _ => panic!("failed to load atomic"),
            }
        }
    }

    /// Subtracts the value from the `Counter` using `Ordering::Relaxed`
    pub fn decrement(&self, value: T) {
        unsafe {
            match std::mem::size_of::<T>() {
                1 if std::mem::align_of::<T>() >= 1 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU8))
                        .fetch_sub(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                2 if std::mem::align_of::<T>() >= 2 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU16))
                        .fetch_sub(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                4 if std::mem::align_of::<T>() >= 4 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU32))
                        .fetch_sub(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                8 if std::mem::align_of::<T>() >= 8 => std::mem::transmute_copy(
                    &(*(self.value.get() as *const AtomicU64))
                        .fetch_sub(std::mem::transmute_copy(&value), Ordering::Relaxed),
                ),
                _ => panic!("failed to load atomic"),
            }
        }
    }

    /// Attempt to subtracts the value from the `Counter` using `Ordering::Relaxed` without overflow
    pub fn try_decrement(&self, value: T) -> Result<(), ()> {
        if self.get() < value {
            Err(())
        } else {
            self.decrement(value);
            Ok(())
        }
    }

    /// Returns `Counter` to its default value using `Ordering::SeqCst`
    pub fn reset(&self) {
        self.set(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_u64() {
        let counter = Counter::<u64>::default();
        assert_eq!(counter.get(), 0);
        counter.set(1);
        assert_eq!(counter.get(), 1);
        counter.increment(1);
        assert_eq!(counter.get(), 2);
        counter.increment(2);
        assert_eq!(counter.get(), 4);
        counter.decrement(1);
        assert_eq!(counter.get(), 3);
        counter.decrement(2);
        assert_eq!(counter.get(), 1);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }
}
