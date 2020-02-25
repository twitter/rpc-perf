use crate::*;
use core::cell::UnsafeCell;
use core::mem::transmute_copy;
use core::sync::atomic::AtomicPtr;

unsafe impl<T> Sync for Atomic<*mut T> {}

impl<T> AtomicNew<*mut T> for Atomic<*mut T> {
    fn new(value: *mut T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

impl<T> AtomicLoad<*mut T> for Atomic<*mut T> {
    fn load(&self, ordering: Ordering) -> *mut T {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicPtr<T>)).load(ordering)) }
    }
}

impl<T> AtomicStore<*mut T> for Atomic<*mut T> {
    fn store(&self, value: *mut T, ordering: Ordering) {
        unsafe {
            transmute_copy(&(*(self.inner.get() as *const AtomicPtr<T>)).store(value, ordering))
        }
    }
}

impl<T> AtomicSwap<*mut T> for Atomic<*mut T> {
    fn swap(&self, new: *mut T, ordering: Ordering) -> *mut T {
        unsafe { transmute_copy(&(*(self.inner.get() as *const AtomicPtr<T>)).swap(new, ordering)) }
    }
}

impl<T> AtomicCompareAndSwap<*mut T> for Atomic<*mut T> {
    fn compare_and_swap(
        &self,
        current: *mut T,
        new: *mut T,
        ordering: Ordering,
    ) -> *mut T {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicPtr<T>))
                    .compare_and_swap(current, new, ordering),
            )
        }
    }
}

impl<T> AtomicCompareExchange<*mut T> for Atomic<*mut T> {
    fn compare_exchange(
        &self,
        current: *mut T,
        new: *mut T,
        success: Ordering,
        failure: Ordering,
    ) -> *mut T {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicPtr<T>))
                    .compare_exchange(current, new, success, failure),
            )
        }
    }
}

impl<T> AtomicCompareExchangeWeak<*mut T> for Atomic<*mut T> {
    fn compare_exchange_weak(
        &self,
        current: *mut T,
        new: *mut T,
        success: Ordering,
        failure: Ordering,
    ) -> *mut T {
        unsafe {
            transmute_copy(
                &(*(self.inner.get() as *const AtomicPtr<T>))
                    .compare_exchange_weak(current, new, success, failure),
            )
        }
    }
}
