use crate::atomic_counter::AtomicCounter;
use core::sync::atomic::{AtomicI8, Ordering};

impl AtomicCounter for AtomicI8 {
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

impl From<i8> for Box<AtomicCounter<Primitive = i8>> {
    fn from(value: i8) -> Box<AtomicCounter<Primitive = i8>> {
        Box::new(AtomicI8::new(value))
    }
}
