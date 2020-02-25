use crate::*;
use core::mem::transmute_copy;

use core::cell::UnsafeCell;
use core::sync::atomic::AtomicIsize;

unsafe impl Sync for Atomic<isize> {}

// impl Default for Atomic<isize> {
//     fn default() -> Self {
//         Self::new(isize::default())
//     }
// }

impl AtomicNew<isize> for Atomic<isize> {
    fn new(value: isize) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<isize> for Atomic<isize> {
    fn load(&self, ordering: Ordering) -> isize {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).load(ordering)) }
    }
}

impl AtomicStore<isize> for Atomic<isize> {
    fn store(&self, value: isize, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).store(value, ordering)) }
    }
}

impl AtomicSwap<isize> for Atomic<isize> {
    fn swap(&self, new: isize, ordering: Ordering) -> isize {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<isize> for Atomic<isize> {
    fn compare_and_swap(
        &self,
        current: isize,
        new: isize,
        ordering: Ordering,
    ) -> isize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicIsize)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<isize> for Atomic<isize> {
    fn compare_exchange(
        &self,
        current: isize,
        new: isize,
        success: Ordering,
        failure: Ordering,
    ) -> isize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicIsize))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<isize> for Atomic<isize> {
    fn compare_exchange_weak(
        &self,
        current: isize,
        new: isize,
        success: Ordering,
        failure: Ordering,
    ) -> isize {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicIsize))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<isize> for Atomic<isize> {
    fn fetch_add(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<isize> for Atomic<isize> {
    fn fetch_sub(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<isize> for Atomic<isize> {
    fn fetch_and(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<isize> for Atomic<isize> {
    fn fetch_nand(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<isize> for Atomic<isize> {
    fn fetch_or(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<isize> for Atomic<isize> {
    fn fetch_xor(&self, value: isize, ordering: Ordering) -> isize {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicIsize)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<isize> for Atomic<isize> {
    fn saturating_add(&self, value: isize, ordering: Ordering) -> isize {
        let mut current = self.load(ordering);
        if current == std::isize::MAX {
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

impl AtomicSaturatingSub<isize> for Atomic<isize> {
    fn saturating_sub(&self, value: isize, ordering: Ordering) -> isize {
        let mut current = self.load(ordering);
        if current == std::isize::MIN {
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

impl AtomicStoreMax<isize> for Atomic<isize> {
    fn store_max(&self, value: isize, ordering: Ordering) -> isize {
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

impl AtomicStoreMin<isize> for Atomic<isize> {
    fn store_min(&self, value: isize, ordering: Ordering) -> isize {
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
        let shared = Arc::new(Atomic::<isize>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as isize, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as isize);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<isize>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as isize);
    }
}