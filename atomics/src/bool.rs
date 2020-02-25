use crate::*;

use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicBool;

unsafe impl Sync for Atomic<bool> {}

impl AtomicNew<bool> for Atomic<bool> {
    fn new(value: bool) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl AtomicLoad<bool> for Atomic<bool> {
    fn load(&self, ordering: Ordering) -> bool {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicBool)).load(ordering)) }
    }
}

impl AtomicStore<bool> for Atomic<bool> {
    fn store(&self, value: bool, ordering: Ordering) {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicBool)).store(value, ordering))
        }
    }
}

impl AtomicSwap<bool> for Atomic<bool> {
    fn swap(&self, new: bool, ordering: Ordering) -> bool {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicBool)).swap(new, ordering)) }
    }
}

impl AtomicCompareAndSwap<bool> for Atomic<bool> {
    fn compare_and_swap(
        &self,
        current: bool,
        new: bool,
        ordering: Ordering,
    ) -> bool {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicBool))
                    .compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl AtomicCompareExchange<bool> for Atomic<bool> {
    fn compare_exchange(
        &self,
        current: bool,
        new: bool,
        success: Ordering,
        failure: Ordering,
    ) -> bool {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicBool))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl AtomicCompareExchangeWeak<bool> for Atomic<bool> {
    fn compare_exchange_weak(
        &self,
        current: bool,
        new: bool,
        success: Ordering,
        failure: Ordering,
    ) -> bool {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicBool))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}

impl AtomicFetchAnd<bool> for Atomic<bool> {
    fn fetch_and(&self, value: bool, ordering: Ordering) -> bool {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicBool)).fetch_and(value, ordering))
        }
    }
}

impl AtomicFetchNand<bool> for Atomic<bool> {
    fn fetch_nand(&self, value: bool, ordering: Ordering) -> bool {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicBool)).fetch_nand(value, ordering))
        }
    }
}

impl AtomicFetchOr<bool> for Atomic<bool> {
    fn fetch_or(&self, value: bool, ordering: Ordering) -> bool {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicBool)).fetch_or(value, ordering))
        }
    }
}

impl AtomicFetchXor<bool> for Atomic<bool> {
    fn fetch_xor(&self, value: bool, ordering: Ordering) -> bool {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicBool)).fetch_xor(value, ordering))
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
        let shared = Arc::new(Atomic::<bool>::default());
        let mut threads = Vec::new();
        for _id in 0..THREADS {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                shared.store(true, Ordering::Relaxed);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(shared.load(Ordering::Relaxed), true);
    }
}
