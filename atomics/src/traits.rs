use crate::*;

pub trait AtomicPrimitive<T>:
    AtomicNew<T>
    + AtomicLoad<T>
    + AtomicStore<T>
    + AtomicSwap<T>
    + AtomicCompareAndSwap<T>
    + AtomicCompareExchange<T>
    + AtomicCompareExchangeWeak<T>
{
}

impl<T> Default for Atomic<T>
where
    T: Default
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> AtomicPrimitive<T> for Atomic<T>
where
    Atomic<T>: AtomicNew<T> + AtomicLoad<T> + AtomicStore<T> + AtomicSwap<T> + AtomicCompareAndSwap<T> + AtomicCompareExchange<T> + AtomicCompareExchangeWeak<T>
{}

pub trait AtomicNew<T> {
    fn new(value: T) -> Self;
}

pub trait AtomicLoad<T> {
    fn load(&self, ordering: Ordering) -> T;
}

pub trait AtomicStore<T> {
    fn store(&self, value: T, ordering: Ordering);
}

pub trait AtomicSwap<T> {
    fn swap(&self, new: T, ordering: Ordering) -> T;
}

pub trait AtomicCompareAndSwap<T> {
    fn compare_and_swap(
        &self,
        current: T,
        new: T,
        ordering: Ordering,
    ) -> T;
}

pub trait AtomicCompareExchange<T> {
    fn compare_exchange(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> T;
}

pub trait AtomicCompareExchangeWeak<T> {
    fn compare_exchange_weak(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> T;
}

pub trait AtomicNumeric<T>: AtomicPrimitive<T> + AtomicFetchAdd<T> + AtomicFetchSub<T> {}

impl<T> AtomicNumeric<T> for Atomic<T>
where
    Atomic<T>: AtomicPrimitive<T> + AtomicFetchAdd<T> + AtomicFetchSub<T>
{}

pub trait AtomicFetchAdd<T> {
    fn fetch_add(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicFetchSub<T> {
    fn fetch_sub(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicLogical<T>: AtomicPrimitive<T> + AtomicFetchAnd<T> + AtomicFetchNand<T> + AtomicFetchOr<T> + AtomicFetchXor<T> {}

impl<T> AtomicLogical<T> for Atomic<T>
where
    Atomic<T>: AtomicPrimitive<T> + AtomicFetchAnd<T> + AtomicFetchNand<T> + AtomicFetchOr<T> + AtomicFetchXor<T>
{}

pub trait AtomicFetchAnd<T> {
    fn fetch_and(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicFetchNand<T> {
    fn fetch_nand(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicFetchOr<T> {
    fn fetch_or(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicFetchXor<T> {
    fn fetch_xor(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicSaturatingAdd<T> {
    fn saturating_add(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicSaturatingSub<T> {
    fn saturating_sub(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicStoreMax<T> {
    fn store_max(&self, value: T, ordering: Ordering) -> T;
}

pub trait AtomicStoreMin<T> {
    fn store_min(&self, value: T, ordering: Ordering) -> T;
}
