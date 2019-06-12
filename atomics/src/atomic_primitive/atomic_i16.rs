use crate::atomic_primitive::AtomicPrimitive;
use core::sync::atomic::{AtomicI16, Ordering};
impl AtomicPrimitive for AtomicI16 {
    type Primitive = i16;
    fn get_mut(&mut self) -> &mut Self::Primitive {
        self.get_mut()
    }
    fn into_inner(self) -> Self::Primitive {
        self.into_inner()
    }
    fn load(&self, order: Ordering) -> Self::Primitive {
        self.load(order)
    }
    fn store(&self, value: Self::Primitive, order: Ordering) {
        self.store(value, order);
    }
    fn swap(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.swap(value, order)
    }
    fn compare_and_swap(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        order: Ordering,
    ) -> Self::Primitive {
        self.compare_and_swap(current, new, order)
    }
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.compare_exchange(current, new, success, failure)
    }
    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.compare_exchange_weak(current, new, success, failure)
    }
}

impl From<i16> for Box<AtomicPrimitive<Primitive = i16>> {
    fn from(value: i16) -> Box<AtomicPrimitive<Primitive = i16>> {
        Box::new(AtomicI16::new(value))
    }
}
