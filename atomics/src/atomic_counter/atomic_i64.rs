use crate::atomic_counter::AtomicCounter;
use core::sync::atomic::{AtomicI64, Ordering};

impl AtomicCounter for AtomicI64 {
    fn fetch_add(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_add(value, order)
    }
    fn fetch_sub(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_sub(value, order)
    }
    fn fetch_and(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_and(value, order)
    }
    fn fetch_nand(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_nand(value, order)
    }
    fn fetch_or(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_or(value, order)
    }
    fn fetch_xor(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.fetch_xor(value, order)
    }
}

impl From<i64> for Box<AtomicCounter<Primitive = i64>> {
    fn from(value: i64) -> Box<AtomicCounter<Primitive = i64>> {
        Box::new(AtomicI64::new(value))
    }
}
