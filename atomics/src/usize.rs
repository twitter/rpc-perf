use crate::*;
use core::mem::transmute_copy;

use core::cell::UnsafeCell;
use core::sync::atomic::AtomicUsize;

unsafe impl Sync for Atomic<usize> {}

// impl Default for Atomic<usize> {
//     fn default() -> Self {
//         Self::new(0)
//     }
// }

impl AtomicNew<usize> for Atomic<usize> {
    fn new(value: usize) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<usize> for Atomic<usize> {
    fn load(&self, ordering: Ordering) -> usize {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).load(ordering)) }
    }
}

impl AtomicStore<usize> for Atomic<usize> {
    fn store(&self, value: usize, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).store(value, ordering)) }
    }
}

impl AtomicSwap<usize> for Atomic<usize> {
    fn swap(&self, new: usize, ordering: Ordering) -> usize {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<usize> for Atomic<usize> {
    fn compare_and_swap(
        &self,
        current: usize,
        new: usize,
        ordering: Ordering,
    ) -> usize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicUsize)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<usize> for Atomic<usize> {
    fn compare_exchange(
        &self,
        current: usize,
        new: usize,
        success: Ordering,
        failure: Ordering,
    ) -> usize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicUsize))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<usize> for Atomic<usize> {
    fn compare_exchange_weak(
        &self,
        current: usize,
        new: usize,
        success: Ordering,
        failure: Ordering,
    ) -> usize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicUsize))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<usize> for Atomic<usize> {
    fn fetch_add(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<usize> for Atomic<usize> {
    fn fetch_sub(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<usize> for Atomic<usize> {
    fn fetch_and(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<usize> for Atomic<usize> {
    fn fetch_nand(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<usize> for Atomic<usize> {
    fn fetch_or(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<usize> for Atomic<usize> {
    fn fetch_xor(&self, value: usize, ordering: Ordering) -> usize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicUsize)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<usize> for Atomic<usize> {
    fn saturating_add(&self, value: usize, ordering: Ordering) -> usize {
        let mut current = self.load(ordering);
        if current == std::usize::MAX {
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

impl AtomicSaturatingSub<usize> for Atomic<usize> {
    fn saturating_sub(&self, value: usize, ordering: Ordering) -> usize {
        let mut current = self.load(ordering);
        if current == std::usize::MIN {
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

impl AtomicStoreMax<usize> for Atomic<usize> {
    fn store_max(&self, value: usize, ordering: Ordering) -> usize {
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

impl AtomicStoreMin<usize> for Atomic<usize> {
    fn store_min(&self, value: usize, ordering: Ordering) -> usize {
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
        let shared = Arc::new(Atomic::<usize>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as usize, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as usize);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<usize>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as usize);
    }
}
