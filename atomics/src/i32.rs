use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicI32;

unsafe impl Sync for Atomic<i32> {}

// impl Default for Atomic<i32> {
//     fn default() -> Self {
//         Self::new(i32::default())
//     }
// }

impl AtomicNew<i32> for Atomic<i32> {
    fn new(value: i32) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<i32> for Atomic<i32> {
    fn load(&self, ordering: Ordering) -> i32 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI32)).load(ordering)) }
    }
}

impl AtomicStore<i32> for Atomic<i32> {
    fn store(&self, value: i32, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI32)).store(value, ordering)) }
    }
}

impl AtomicSwap<i32> for Atomic<i32> {
    fn swap(&self, new: i32, ordering: Ordering) -> i32 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI32)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<i32> for Atomic<i32> {
    fn compare_and_swap(
        &self,
        current: i32,
        new: i32,
        ordering: Ordering,
    ) -> i32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI32)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<i32> for Atomic<i32> {
    fn compare_exchange(
        &self,
        current: i32,
        new: i32,
        success: Ordering,
        failure: Ordering,
    ) -> i32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI32))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<i32> for Atomic<i32> {
    fn compare_exchange_weak(
        &self,
        current: i32,
        new: i32,
        success: Ordering,
        failure: Ordering,
    ) -> i32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI32))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<i32> for Atomic<i32> {
    fn fetch_add(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<i32> for Atomic<i32> {
    fn fetch_sub(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<i32> for Atomic<i32> {
    fn fetch_and(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<i32> for Atomic<i32> {
    fn fetch_nand(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<i32> for Atomic<i32> {
    fn fetch_or(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<i32> for Atomic<i32> {
    fn fetch_xor(&self, value: i32, ordering: Ordering) -> i32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI32)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<i32> for Atomic<i32> {
    fn saturating_add(&self, value: i32, ordering: Ordering) -> i32 {
        let mut current = self.load(ordering);
        if current == std::i32::MAX {
            return current;
        } else {
            loop {
                let new = current.saturating_add(value);
                let result = self.compare_and_swap(current, new, ordering);
                if result == current {
                    return new;
                }
                current = result;
            }
        }
    }
}

impl AtomicSaturatingSub<i32> for Atomic<i32> {
    fn saturating_sub(&self, value: i32, ordering: Ordering) -> i32 {
        let mut current = self.load(ordering);
        if current == std::i32::MIN {
            return current;
        } else {
            loop {
                let new = current.saturating_sub(value);
                let result = self.compare_and_swap(current, new, ordering);
                if result == current {
                    return new;
                }
                current = result;
            }
        }
    }
}

impl AtomicStoreMax<i32> for Atomic<i32> {
    fn store_max(&self, value: i32, ordering: Ordering) -> i32 {
        let mut current = self.load(ordering);
        if current >= value {
            current
        } else {
            loop {
                let result = self.compare_and_swap(current, value, ordering);
                if result == current {
                    return value;
                }
                current = result;
                if current >= value {
                    return current;
                }
            }
        }
    }
}

impl AtomicStoreMin<i32> for Atomic<i32> {
    fn store_min(&self, value: i32, ordering: Ordering) -> i32 {
        let mut current = self.load(ordering);
        if current <= value {
            current
        } else {
            loop {
                let result = self.compare_and_swap(current, value, ordering);
                if result == current {
                    return value;
                }
                current = result;
                if current <= value {
                    return current;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    const THREADS: usize = 4;
    use super::*;
    use std::sync::Arc;

    #[test]
    fn store() {
        let shared = Arc::new(Atomic::<i32>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as i32, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as i32);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<i32>::default());
        let mut threads = Vec::new();
        for _ in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.fetch_add(1, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as i32);
    }
}
