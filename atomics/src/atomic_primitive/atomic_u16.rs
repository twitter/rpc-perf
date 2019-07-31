use crate::*;

/// An integer type which can be safely shared between threads.
pub struct AtomicU16 {
    pub(crate) inner: core::sync::atomic::AtomicU16,
}

impl AtomicPrimitive for AtomicU16 {
    type Primitive = u16;
    fn new(value: Self::Primitive) -> Self {
        Self {
            inner: core::sync::atomic::AtomicU16::new(value),
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

impl Default for AtomicU16 {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl PartialEq for AtomicU16 {
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl Eq for AtomicU16 {}
