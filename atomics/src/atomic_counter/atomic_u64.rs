use crate::*;

use crate::atomic_primitive::AtomicU64;

impl AtomicCounter for AtomicU64 {
    fn fetch_add(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_add(value, order)
    }

    fn fetch_sub(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_sub(value, order)
    }

    fn fetch_and(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_and(value, order)
    }

    fn fetch_nand(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_nand(value, order)
    }

    fn fetch_or(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_or(value, order)
    }

    fn fetch_xor(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_xor(value, order)
    }
}
