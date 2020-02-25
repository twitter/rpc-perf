use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicI8;

unsafe impl Sync for Atomic<i8> {}

// impl Default for Atomic<i8> {
//     fn default() -> Self {
//         Self::new(i8::default())
//     }
// }

impl AtomicNew<i8> for Atomic<i8> {
    fn new(value: i8) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<i8> for Atomic<i8> {
    fn load(&self, ordering: Ordering) -> i8 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI8)).load(ordering)) }
    }
}

impl AtomicStore<i8> for Atomic<i8> {
    fn store(&self, value: i8, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI8)).store(value, ordering)) }
    }
}

impl AtomicSwap<i8> for Atomic<i8> {
    fn swap(&self, new: i8, ordering: Ordering) -> i8 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI8)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<i8> for Atomic<i8> {
    fn compare_and_swap(
        &self,
        current: i8,
        new: i8,
        ordering: Ordering,
    ) -> i8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI8)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<i8> for Atomic<i8> {
    fn compare_exchange(
        &self,
        current: i8,
        new: i8,
        success: Ordering,
        failure: Ordering,
    ) -> i8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI8))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<i8> for Atomic<i8> {
    fn compare_exchange_weak(
        &self,
        current: i8,
        new: i8,
        success: Ordering,
        failure: Ordering,
    ) -> i8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI8))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<i8> for Atomic<i8> {
    fn fetch_add(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<i8> for Atomic<i8> {
    fn fetch_sub(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<i8> for Atomic<i8> {
    fn fetch_and(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<i8> for Atomic<i8> {
    fn fetch_nand(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<i8> for Atomic<i8> {
    fn fetch_or(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<i8> for Atomic<i8> {
    fn fetch_xor(&self, value: i8, ordering: Ordering) -> i8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI8)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<i8> for Atomic<i8> {
    fn saturating_add(&self, value: i8, ordering: Ordering) -> i8 {
        let mut current = self.load(ordering);
        if current == std::i8::MAX {
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

impl AtomicSaturatingSub<i8> for Atomic<i8> {
    fn saturating_sub(&self, value: i8, ordering: Ordering) -> i8 {
        let mut current = self.load(ordering);
        if current == std::i8::MIN {
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

impl AtomicStoreMax<i8> for Atomic<i8> {
    fn store_max(&self, value: i8, ordering: Ordering) -> i8 {
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

impl AtomicStoreMin<i8> for Atomic<i8> {
    fn store_min(&self, value: i8, ordering: Ordering) -> i8 {
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
        let shared = Arc::new(Atomic::<i8>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as i8, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as i8);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<i8>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as i8);
    }
}
