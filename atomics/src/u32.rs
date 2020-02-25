use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicU32;

unsafe impl Sync for Atomic<u32> {}

// impl Default for Atomic<u32> {
//     fn default() -> Self {
//         Self::new(0)
//     }
// }

impl AtomicNew<u32> for Atomic<u32> {
    fn new(value: u32) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<u32> for Atomic<u32> {
    fn load(&self, ordering: Ordering) -> u32 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU32)).load(ordering)) }
    }
}

impl AtomicStore<u32> for Atomic<u32> {
    fn store(&self, value: u32, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU32)).store(value, ordering)) }
    }
}

impl AtomicSwap<u32> for Atomic<u32> {
    fn swap(&self, new: u32, ordering: Ordering) -> u32 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU32)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<u32> for Atomic<u32> {
    fn compare_and_swap(
        &self,
        current: u32,
        new: u32,
        ordering: Ordering,
    ) -> u32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU32)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<u32> for Atomic<u32> {
    fn compare_exchange(
        &self,
        current: u32,
        new: u32,
        success: Ordering,
        failure: Ordering,
    ) -> u32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU32))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<u32> for Atomic<u32> {
    fn compare_exchange_weak(
        &self,
        current: u32,
        new: u32,
        success: Ordering,
        failure: Ordering,
    ) -> u32 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU32))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<u32> for Atomic<u32> {
    fn fetch_add(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<u32> for Atomic<u32> {
    fn fetch_sub(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<u32> for Atomic<u32> {
    fn fetch_and(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<u32> for Atomic<u32> {
    fn fetch_nand(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<u32> for Atomic<u32> {
    fn fetch_or(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<u32> for Atomic<u32> {
    fn fetch_xor(&self, value: u32, ordering: Ordering) -> u32 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU32)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<u32> for Atomic<u32> {
    fn saturating_add(&self, value: u32, ordering: Ordering) -> u32 {
        let mut current = self.load(ordering);
        if current == std::u32::MAX {
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

impl AtomicSaturatingSub<u32> for Atomic<u32> {
    fn saturating_sub(&self, value: u32, ordering: Ordering) -> u32 {
        let mut current = self.load(ordering);
        if current == std::u32::MIN {
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

impl AtomicStoreMax<u32> for Atomic<u32> {
    fn store_max(&self, value: u32, ordering: Ordering) -> u32 {
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

impl AtomicStoreMin<u32> for Atomic<u32> {
    fn store_min(&self, value: u32, ordering: Ordering) -> u32 {
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
        let shared = Arc::new(Atomic::<u32>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as u32, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as u32);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<u32>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as u32);
    }
}