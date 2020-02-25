use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicU8;

unsafe impl Sync for Atomic<u8> {}

// impl Default for Atomic<u8> {
//     fn default() -> Self {
//         Self::new(0)
//     }
// }

impl AtomicNew<u8> for Atomic<u8> {
    fn new(value: u8) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<u8> for Atomic<u8> {
    fn load(&self, ordering: Ordering) -> u8 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU8)).load(ordering)) }
    }
}

impl AtomicStore<u8> for Atomic<u8> {
    fn store(&self, value: u8, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU8)).store(value, ordering)) }
    }
}

impl AtomicSwap<u8> for Atomic<u8> {
    fn swap(&self, new: u8, ordering: Ordering) -> u8 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU8)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<u8> for Atomic<u8> {
    fn compare_and_swap(
        &self,
        current: u8,
        new: u8,
        ordering: Ordering,
    ) -> u8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU8)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<u8> for Atomic<u8> {
    fn compare_exchange(
        &self,
        current: u8,
        new: u8,
        success: Ordering,
        failure: Ordering,
    ) -> u8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU8))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<u8> for Atomic<u8> {
    fn compare_exchange_weak(
        &self,
        current: u8,
        new: u8,
        success: Ordering,
        failure: Ordering,
    ) -> u8 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU8))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<u8> for Atomic<u8> {
    fn fetch_add(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<u8> for Atomic<u8> {
    fn fetch_sub(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<u8> for Atomic<u8> {
    fn fetch_and(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<u8> for Atomic<u8> {
    fn fetch_nand(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<u8> for Atomic<u8> {
    fn fetch_or(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<u8> for Atomic<u8> {
    fn fetch_xor(&self, value: u8, ordering: Ordering) -> u8 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU8)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<u8> for Atomic<u8> {
    fn saturating_add(&self, value: u8, ordering: Ordering) -> u8 {
        let mut current = self.load(ordering);
        if current == std::u8::MAX {
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

impl AtomicSaturatingSub<u8> for Atomic<u8> {
    fn saturating_sub(&self, value: u8, ordering: Ordering) -> u8 {
        let mut current = self.load(ordering);
        if current == std::u8::MIN {
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

impl AtomicStoreMax<u8> for Atomic<u8> {
    fn store_max(&self, value: u8, ordering: Ordering) -> u8 {
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

impl AtomicStoreMin<u8> for Atomic<u8> {
    fn store_min(&self, value: u8, ordering: Ordering) -> u8 {
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
        let shared = Arc::new(Atomic::<u8>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as u8, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as u8);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<u8>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as u8);
    }
}
