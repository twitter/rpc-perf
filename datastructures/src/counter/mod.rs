use atomics::*;
use core::sync::atomic::Ordering;

mod counter_primitive;

pub use self::counter_primitive::*;

pub struct Counter<T> {
    inner: Box<AtomicCounter<Primitive = T>>,
}

impl<T> Default for Counter<T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: CounterPrimitive + Default,
{
    fn default() -> Self {
        Self {
            inner: Box::<AtomicCounter<Primitive = T>>::from(T::default()),
        }
    }
}

impl<T> Counter<T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: CounterPrimitive + Default,
{
    /// Create new counter from primitive
    pub fn new(value: T) -> Counter<T> {
        Self {
            inner: Box::<AtomicCounter<Primitive = T>>::from(value),
        }
    }

    /// Atomic load
    pub fn load(&self, order: Ordering) -> T {
        self.inner.load(order)
    }

    /// Atomic store
    pub fn store(&self, value: T, order: Ordering) {
        self.inner.store(value, order)
    }

    /// Atomic compare and swap
    pub fn compare_and_swap(&self, current: T, new: T, order: Ordering) -> T {
        self.inner.compare_and_swap(current, new, order)
    }

    /// Atomic fetch add
    pub fn fetch_add(&self, value: T, order: Ordering) -> T {
        self.inner.fetch_add(value, order)
    }

    /// Atomic fetch sub
    pub fn fetch_sub(&self, value: T, order: Ordering) -> T {
        self.inner.fetch_sub(value, order)
    }

    /// Convenience function to do a relaxed read
    pub fn get(&self) -> T {
        self.load(Ordering::Relaxed)
    }

    /// Convenience function to do a squentially consistent write
    pub fn set(&self, value: T) {
        self.store(value, Ordering::SeqCst)
    }

    /// Convenience function to do a relaxed wrapping add
    pub fn add(&self, value: T) -> T {
        self.fetch_add(value, Ordering::Relaxed)
    }

    /// Convenience function to do a relaxed wrapping sub
    pub fn sub(&self, value: T) -> T {
        self.fetch_sub(value, Ordering::Relaxed)
    }

    /// Saturating add using atomic intrinsics
    pub fn saturating_add(&self, value: T) -> T {
        let mut current = self.load(Ordering::SeqCst);
        let mut new = current.saturating_add(value);
        loop {
            let result = self.compare_and_swap(current, new, Ordering::SeqCst);
            if result == current {
                return current;
            }
            new = result.saturating_add(value);
            current = result;
        }
    }

    /// Saturating sub using atomic intrinsics
    pub fn saturating_sub(&self, value: T) -> T {
        let mut current = self.load(Ordering::SeqCst);
        let mut new = current.saturating_sub(value);
        loop {
            let result = self.compare_and_swap(current, new, Ordering::SeqCst);
            if result == current {
                return current;
            }
            new = result.saturating_sub(value);
            current = result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
    }

    #[test]
    fn store() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.store(42, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), 42);
        assert_eq!(c.get(), 42);
    }

    #[test]
    fn compare_and_swap() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        assert_eq!(0, c.compare_and_swap(0, 42, Ordering::SeqCst));
        assert_eq!(c.load(Ordering::SeqCst), 42);
        assert_eq!(c.get(), 42);
    }

    #[test]
    fn fetch_add() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.fetch_add(1, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn fetch_sub() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
    }

    #[test]
    fn add() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.add(1);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn sub() {
        let c: Counter<usize> = Counter::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.sub(1);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
    }

    #[test]
    fn saturating_add() {
        let c: Counter<usize> = Counter::new(usize::max_value() - 1);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value() - 1);
        assert_eq!(c.get(), usize::max_value() - 1);
        c.saturating_add(1);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
        c.saturating_add(1);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
    }

    #[test]
    fn saturating_sub() {
        let c: Counter<usize> = Counter::new(1);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        assert_eq!(c.get(), 1);
        c.saturating_sub(1);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.saturating_sub(1);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
    }
}
