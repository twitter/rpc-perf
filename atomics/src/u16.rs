use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicU16;

unsafe impl Sync for Atomic<u16> {}

// impl Default for Atomic<u16> {
//     fn default() -> Self {
//         Self::new(0)
//     }
// }

impl AtomicNew<u16> for Atomic<u16> {
    fn new(value: u16) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<u16> for Atomic<u16> {
    fn load(&self, ordering: Ordering) -> u16 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU16)).load(ordering)) }
    }
}

impl AtomicStore<u16> for Atomic<u16> {
    fn store(&self, value: u16, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU16)).store(value, ordering)) }
    }
}

impl AtomicSwap<u16> for Atomic<u16> {
    fn swap(&self, new: u16, ordering: Ordering) -> u16 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicU16)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<u16> for Atomic<u16> {
    fn compare_and_swap(
        &self,
        current: u16,
        new: u16,
        ordering: Ordering,
    ) -> u16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU16)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<u16> for Atomic<u16> {
    fn compare_exchange(
        &self,
        current: u16,
        new: u16,
        success: Ordering,
        failure: Ordering,
    ) -> u16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU16))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<u16> for Atomic<u16> {
    fn compare_exchange_weak(
        &self,
        current: u16,
        new: u16,
        success: Ordering,
        failure: Ordering,
    ) -> u16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicU16))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<u16> for Atomic<u16> {
    fn fetch_add(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<u16> for Atomic<u16> {
    fn fetch_sub(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<u16> for Atomic<u16> {
    fn fetch_and(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<u16> for Atomic<u16> {
    fn fetch_nand(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<u16> for Atomic<u16> {
    fn fetch_or(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<u16> for Atomic<u16> {
    fn fetch_xor(&self, value: u16, ordering: Ordering) -> u16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicU16)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<u16> for Atomic<u16> {
    fn saturating_add(&self, value: u16, ordering: Ordering) -> u16 {
        let mut current = self.load(ordering);
        if current == std::u16::MAX {
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

impl AtomicSaturatingSub<u16> for Atomic<u16> {
    fn saturating_sub(&self, value: u16, ordering: Ordering) -> u16 {
        let mut current = self.load(ordering);
        if current == std::u16::MIN {
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

impl AtomicStoreMax<u16> for Atomic<u16> {
    fn store_max(&self, value: u16, ordering: Ordering) -> u16 {
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

impl AtomicStoreMin<u16> for Atomic<u16> {
    fn store_min(&self, value: u16, ordering: Ordering) -> u16 {
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
        let shared = Arc::new(Atomic::<u16>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as u16, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as u16);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<u16>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as u16);
    }
}
