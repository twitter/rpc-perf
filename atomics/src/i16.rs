use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicI16;

unsafe impl Sync for Atomic<i16> {}

// impl Default for Atomic<i16> {
//     fn default() -> Self {
//         Self::new(i16::default())
//     }
// }

impl AtomicNew<i16> for Atomic<i16> {
    fn new(value: i16) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<i16> for Atomic<i16> {
    fn load(&self, ordering: Ordering) -> i16 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI16)).load(ordering)) }
    }
}

impl AtomicStore<i16> for Atomic<i16> {
    fn store(&self, value: i16, ordering: Ordering) {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI16)).store(value, ordering)) }
    }
}

impl AtomicSwap<i16> for Atomic<i16> {
    fn swap(&self, new: i16, ordering: Ordering) -> i16 {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicI16)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<i16> for Atomic<i16> {
    fn compare_and_swap(
        &self,
        current: i16,
        new: i16,
        ordering: Ordering,
    ) -> i16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI16)).compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<i16> for Atomic<i16> {
    fn compare_exchange(
        &self,
        current: i16,
        new: i16,
        success: Ordering,
        failure: Ordering,
    ) -> i16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI16))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<i16> for Atomic<i16> {
    fn compare_exchange_weak(
        &self,
        current: i16,
        new: i16,
        success: Ordering,
        failure: Ordering,
    ) -> i16 {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicI16))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAdd<i16> for Atomic<i16> {
    fn fetch_add(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_add(value, ordering))
        }
    }
}

impl AtomicFetchSub<i16> for Atomic<i16> {
    fn fetch_sub(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_sub(value, ordering))
        }
    }
}

impl AtomicFetchAnd<i16> for Atomic<i16> {
    fn fetch_and(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<i16> for Atomic<i16> {
    fn fetch_nand(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<i16> for Atomic<i16> {
    fn fetch_or(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<i16> for Atomic<i16> {
    fn fetch_xor(&self, value: i16, ordering: Ordering) -> i16 {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicI16)).fetch_xor(value, ordering))
        }
    }
}

impl AtomicSaturatingAdd<i16> for Atomic<i16> {
    fn saturating_add(&self, value: i16, ordering: Ordering) -> i16 {
        let mut current = self.load(ordering);
        if current == std::i16::MAX {
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

impl AtomicSaturatingSub<i16> for Atomic<i16> {
    fn saturating_sub(&self, value: i16, ordering: Ordering) -> i16 {
        let mut current = self.load(ordering);
        if current == std::i16::MIN {
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

impl AtomicStoreMax<i16> for Atomic<i16> {
    fn store_max(&self, value: i16, ordering: Ordering) -> i16 {
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

impl AtomicStoreMin<i16> for Atomic<i16> {
    fn store_min(&self, value: i16, ordering: Ordering) -> i16 {
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
        let shared = Arc::new(Atomic::<i16>::default());
        let mut threads = Vec::new();
        for id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(id as i16, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(shared.load(Ordering::Relaxed) < THREADS as i16);
    }

    #[test]
    fn fetch_add() {
        let shared = Arc::new(Atomic::<i16>::default());
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
        assert_eq!(shared.load(Ordering::Relaxed), THREADS as i16);
    }
}
