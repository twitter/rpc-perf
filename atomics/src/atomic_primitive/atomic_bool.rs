use crate::*;

pub struct AtomicBool {
    pub(crate) inner: core::sync::atomic::AtomicBool,
}

impl AtomicPrimitive for AtomicBool {
    type Primitive = bool;
    fn new(value: Self::Primitive) -> Self {
        Self {
            inner: core::sync::atomic::AtomicBool::new(value),
        }
    }
    fn get_mut(&mut self) -> &mut Self::Primitive {
        self.inner.get_mut()
    }
    fn into_inner(self) -> Self::Primitive {
        self.inner.into_inner()
    }
    fn load(&self, order: Ordering) -> Self::Primitive {
        self.inner.load(order)
    }
    fn store(&self, value: Self::Primitive, order: Ordering) {
        self.inner.store(value, order);
    }
    fn swap(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.swap(value, order)
    }
    fn compare_and_swap(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        order: Ordering,
    ) -> Self::Primitive {
        self.inner.compare_and_swap(current, new, order)
    }
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.inner.compare_exchange(current, new, success, failure)
    }
    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.inner
            .compare_exchange_weak(current, new, success, failure)
    }
}

impl Default for AtomicBool {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
