use crate::atomic_counter::AtomicCounter;
use core::sync::atomic::{AtomicU64, Ordering};

impl AtomicCounter for AtomicU64 {
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

impl From<u64> for Box<AtomicCounter<Primitive = u64>> {
    fn from(value: u64) -> Box<AtomicCounter<Primitive = u64>> {
        Box::new(AtomicU64::new(value))
    }
}
