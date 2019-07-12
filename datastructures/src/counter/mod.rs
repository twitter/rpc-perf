use atomics::*;

pub trait Unsigned {}
impl Unsigned for AtomicU8 {}
impl Unsigned for AtomicU16 {}
impl Unsigned for AtomicU32 {}
impl Unsigned for AtomicU64 {}
impl Unsigned for AtomicUsize {}

pub trait Saturating {
    fn saturating_add(&self, other: Self) -> Self;
    fn saturating_sub(&self, other: Self) -> Self;
}

impl Saturating for i8 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i8).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i8).saturating_sub(other)
    }
}

impl Saturating for i16 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i16).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i16).saturating_sub(other)
    }
}

impl Saturating for i32 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i32).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i32).saturating_sub(other)
    }
}

impl Saturating for i64 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i64).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i64).saturating_sub(other)
    }
}

impl Saturating for isize {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as isize).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as isize).saturating_sub(other)
    }
}

impl Saturating for u8 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u8).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u8).saturating_sub(other)
    }
}

impl Saturating for u16 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u16).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u16).saturating_sub(other)
    }
}

impl Saturating for u32 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u32).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u32).saturating_sub(other)
    }
}

impl Saturating for u64 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u64).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u64).saturating_sub(other)
    }
}

impl Saturating for usize {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as usize).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as usize).saturating_sub(other)
    }
}

pub trait Counter: Default + AtomicCounter + AtomicPrimitive
where
    <Self as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
{
    /// Convenience function to do a relaxed read
    fn get(&self) -> Self::Primitive {
        self.load(Ordering::Relaxed)
    }

    /// Convenience function to do a squentially consistent write
    fn set(&self, value: Self::Primitive) {
        self.store(value, Ordering::SeqCst)
    }

    /// Convenience function to do a relaxed wrapping add
    fn add(&self, value: Self::Primitive) -> Self::Primitive {
        self.fetch_add(value, Ordering::Relaxed)
    }

    /// Convenience function to do a relaxed wrapping sub
    fn sub(&self, value: Self::Primitive) -> Self::Primitive {
        self.fetch_sub(value, Ordering::Relaxed)
    }

    /// Saturating add using atomic intrinsics
    fn saturating_add(&self, value: Self::Primitive) -> Self::Primitive {
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
    fn saturating_sub(&self, value: Self::Primitive) -> Self::Primitive {
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

impl Counter for AtomicI8 {}
impl Counter for AtomicI16 {}
impl Counter for AtomicI32 {}
impl Counter for AtomicI64 {}
impl Counter for AtomicIsize {}
impl Counter for AtomicU8 {}
impl Counter for AtomicU16 {}
impl Counter for AtomicU32 {}
impl Counter for AtomicU64 {}
impl Counter for AtomicUsize {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn new() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
    }

    #[test]
    fn sizes() {
        assert_eq!(size_of::<AtomicU8>(), 1);
        assert_eq!(size_of::<AtomicU16>(), 2);
        assert_eq!(size_of::<AtomicU32>(), 4);
        assert_eq!(size_of::<AtomicU64>(), 8);
    }

    #[test]
    fn store() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.store(42, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), 42);
        assert_eq!(c.get(), 42);
    }

    #[test]
    fn compare_and_swap() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        assert_eq!(0, c.compare_and_swap(0, 42, Ordering::SeqCst));
        assert_eq!(c.load(Ordering::SeqCst), 42);
        assert_eq!(c.get(), 42);
    }

    #[test]
    fn fetch_add() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.fetch_add(1, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn fetch_sub() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
    }

    #[test]
    fn add() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.add(1);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn sub() {
        let c = AtomicUsize::new(0);
        assert_eq!(c.load(Ordering::SeqCst), 0);
        assert_eq!(c.get(), 0);
        c.sub(1);
        assert_eq!(c.load(Ordering::SeqCst), usize::max_value());
        assert_eq!(c.get(), usize::max_value());
    }

    #[test]
    fn saturating_add() {
        let c = AtomicUsize::new(usize::max_value() - 1);
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
        let c = AtomicUsize::new(1);
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
