use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicU64;

unsafe impl Sync for Atomic<u64> {}

// impl Default for Atomic<u64> {
//     fn default() -> Self {
//         Self::new(0)
//     }
// }

impl AtomicNew<u64> for Atomic<u64> {
    fn new(value: u64) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<u64> for Atomic<u64> {
    fn load(&self, ordering: Ordering) -> u64 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU64)).load(ordering)) }
    }
}

impl AtomicStore<u64> for Atomic<u64> {
    fn store(&self, value: u64, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU64)).store(value, ordering)) }
    }
}

impl AtomicSwap<u64> for Atomic<u64> {
    fn swap(&self, new: u64, ordering: Ordering) -> u64 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU64)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<u64> for Atomic<u64> {
    fn compare_and_swap(
        &self,
        current: u64,
        new: u64,
        ordering: Ordering,
    ) -> u64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU64)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<u64> for Atomic<u64> {
    fn compare_exchange(
        &self,
        current: u64,
        new: u64,
        success: Ordering,
        failure: Ordering,
    ) -> u64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU64))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<u64> for Atomic<u64> {
    fn compare_exchange_weak(
        &self,
        current: u64,
        new: u64,
        success: Ordering,
        failure: Ordering,
    ) -> u64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU64))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<u64> for Atomic<u64> {
    fn fetch_add(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<u64> for Atomic<u64> {
    fn fetch_sub(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<u64> for Atomic<u64> {
    fn fetch_and(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<u64> for Atomic<u64> {
    fn fetch_nand(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<u64> for Atomic<u64> {
    fn fetch_or(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<u64> for Atomic<u64> {
    fn fetch_xor(&self, value: u64, ordering: Ordering) -> u64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU64)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<u64> for Atomic<u64> {
    fn saturating_add(&self, value: u64, ordering: Ordering) -> u64 {
        let mut current = self.load(ordering);
        if current == std::u64::MAX {
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

impl AtomicSaturatingSub<u64> for Atomic<u64> {
    fn saturating_sub(&self, value: u64, ordering: Ordering) -> u64 {
        let mut current = self.load(ordering);
        if current == std::u64::MIN {
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

impl AtomicStoreMax<u64> for Atomic<u64> {
    fn store_max(&self, value: u64, ordering: Ordering) -> u64 {
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

impl AtomicStoreMin<u64> for Atomic<u64> {
    fn store_min(&self, value: u64, ordering: Ordering) -> u64 {
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
        let shared = Arc::new(Atomic::<u64>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as u64, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as u64);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<u64>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as u64);
    }
}
