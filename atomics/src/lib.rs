use core::sync::atomic::Ordering;

mod atomic_counter;
mod atomic_primitive;

pub use crate::atomic_counter::*;
pub use crate::atomic_primitive::*;

pub struct Atomic<T> {
    inner: Box<AtomicPrimitive<Primitive = T>>,
}

impl<T> Atomic<T>
where
    Box<AtomicPrimitive<Primitive = T>>: From<T>,
{
    pub fn new(value: T) -> Atomic<T> {
        Self {
            inner: Box::<AtomicPrimitive<Primitive = T>>::from(value),
        }
    }

    pub fn load(&self, order: Ordering) -> T {
        self.inner.load(order)
    }

    pub fn store(&self, value: T, order: Ordering) {
        self.inner.store(value, order)
    }

    pub fn swap(&self, value: T, order: Ordering) -> T {
        self.inner.swap(value, order)
    }

    pub fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T {
        self.inner.compare_and_swap(current, new, order)
    }

    pub fn compare_exchange(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, T> {
        self.inner.compare_exchange(current, new, success, failure)
    }

    pub fn compare_exchange_weak(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, T> {
        self.inner
            .compare_exchange_weak(current, new, success, failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize() {
        let x = Box::<AtomicPrimitive<Primitive = usize>>::from(0);
        assert_eq!(x.load(Ordering::SeqCst), 0_usize);
        x.store(42, Ordering::SeqCst);
        assert_eq!(x.load(Ordering::SeqCst), 42_usize);
    }

    #[test]
    fn new_counters() {
        let x = Box::<AtomicCounter<Primitive = usize>>::from(0);
        assert_eq!(x.load(Ordering::SeqCst), 0_usize);
    }

    #[test]
    fn new_atomic_usize() {
        let x: Atomic<usize> = Atomic::new(0_usize);
        assert_eq!(x.load(Ordering::SeqCst), 0_usize);
        x.store(42, Ordering::SeqCst);
        assert_eq!(x.load(Ordering::SeqCst), 42_usize);
    }
}
