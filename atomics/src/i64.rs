use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicI64;

unsafe impl Sync for Atomic<i64> {}

// impl Default for Atomic<i64> {
//     fn default() -> Self {
//         Self::new(i64::default())
//     }
// }

impl AtomicNew<i64> for Atomic<i64> {
    fn new(value: i64) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<i64> for Atomic<i64> {
    fn load(&self, ordering: Ordering) -> i64 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI64)).load(ordering)) }
    }
}

impl AtomicStore<i64> for Atomic<i64> {
    fn store(&self, value: i64, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI64)).store(value, ordering)) }
    }
}

impl AtomicSwap<i64> for Atomic<i64> {
    fn swap(&self, new: i64, ordering: Ordering) -> i64 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI64)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<i64> for Atomic<i64> {
    fn compare_and_swap(
        &self,
        current: i64,
        new: i64,
        ordering: Ordering,
    ) -> i64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI64)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<i64> for Atomic<i64> {
    fn compare_exchange(
        &self,
        current: i64,
        new: i64,
        success: Ordering,
        failure: Ordering,
    ) -> i64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI64))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<i64> for Atomic<i64> {
    fn compare_exchange_weak(
        &self,
        current: i64,
        new: i64,
        success: Ordering,
        failure: Ordering,
    ) -> i64 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI64))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<i64> for Atomic<i64> {
    fn fetch_add(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<i64> for Atomic<i64> {
    fn fetch_sub(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<i64> for Atomic<i64> {
    fn fetch_and(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<i64> for Atomic<i64> {
    fn fetch_nand(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<i64> for Atomic<i64> {
    fn fetch_or(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<i64> for Atomic<i64> {
    fn fetch_xor(&self, value: i64, ordering: Ordering) -> i64 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI64)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<i64> for Atomic<i64> {
    fn saturating_add(&self, value: i64, ordering: Ordering) -> i64 {
        let mut current = self.load(ordering);
        if current == std::i64::MAX {
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

impl AtomicSaturatingSub<i64> for Atomic<i64> {
    fn saturating_sub(&self, value: i64, ordering: Ordering) -> i64 {
        let mut current = self.load(ordering);
        if current == std::i64::MIN {
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

impl AtomicStoreMax<i64> for Atomic<i64> {
    fn store_max(&self, value: i64, ordering: Ordering) -> i64 {
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

impl AtomicStoreMin<i64> for Atomic<i64> {
    fn store_min(&self, value: i64, ordering: Ordering) -> i64 {
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
        let shared = Arc::new(Atomic::<i64>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as i64, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as i64);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<i64>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as i64);
    }
}
