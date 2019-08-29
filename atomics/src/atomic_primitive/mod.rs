// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Ordering;

use core::fmt::Debug;

mod atomic_bool;
mod atomic_i16;
mod atomic_i32;
mod atomic_i64;
mod atomic_i8;
mod atomic_isize;
mod atomic_ptr;
mod atomic_u16;
mod atomic_u32;
mod atomic_u64;
mod atomic_u8;
mod atomic_usize;

pub use self::atomic_bool::*;
pub use self::atomic_i16::*;
pub use self::atomic_i32::*;
pub use self::atomic_i64::*;
pub use self::atomic_i8::*;
pub use self::atomic_isize::*;
pub use self::atomic_ptr::*;
pub use self::atomic_u16::*;
pub use self::atomic_u32::*;
pub use self::atomic_u64::*;
pub use self::atomic_u8::*;
pub use self::atomic_usize::*;

/// This trait is used to define the functions which are available on types
/// which may be used as atomic primitives, allowing for them to be used as
/// generic types.
pub trait AtomicPrimitive: Send + Sync + Debug + PartialEq {
    type Primitive;

    /// Create a new `AtomicPrimitive` from a primitive type
    fn new(value: Self::Primitive) -> Self;

    /// Get a mutable reference to the underlying primitive
    ///
    /// This is safe because the mutable reference guarantees no other threads
    /// are concurrently accessing the atomic data
    fn get_mut(&mut self) -> &mut Self::Primitive;

    /// Consumes the `AtomicPrimitive` and returns the underlying primitive
    ///
    /// This is safe because passing `self` by value guarantees that no other
    /// threads are concurrently accessing the atomic data
    fn into_inner(self) -> Self::Primitive;

    /// Loads the value from the `AtomicPrimitive`
    ///
    /// `load` takes an `Ordering` argument which describes the memory ordering
    /// of this operation. Possible values are `SeqCst`, `Acquire`, and
    /// `Relaxed`
    fn load(&self, order: Ordering) -> Self::Primitive;

    /// Stores a value into the `AtomicPrimitive`
    ///
    /// `store` takes an `Ordering` argument which describes the memory ordering
    /// of this operation. Possible values are `SeqCst`, `Acquire`, `Release`,
    /// and `Relaxed`.
    fn store(&self, value: Self::Primitive, order: Ordering);

    /// Stores a value into the `AtomicPrimitive`, returning the previous value.
    /// `swap` takes an `Ordering` argument which describes the memory ordering
    /// of this operation. All ordering modes are possible. Note that using
    /// `Acquire` makes the store part of this operation `Relaxed`, and using
    /// `Release` makes the load part `Relaxed`.
    fn swap(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Stores a value into the `AtomicPrimitive` if the current value is the
    /// same as the `current` value.
    ///
    /// The return value is always the previous value. If it is equal to
    /// `current`, then the value was updated.
    ///
    /// `compare_and_swap` takes an `Ordering` argument which describes the
    /// memory ordering of this operation. Note that even when using `AcqRel`,
    /// the operation might fail and hence just perform an `Acquire` load, but
    /// not have `Release` semantics. Using `Acquire` makes the store part of
    /// this operation `Relaxed` if it happens, and using `Release` makes the
    /// load part `Relaxed`.
    fn compare_and_swap(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        order: Ordering,
    ) -> Self::Primitive;

    /// Stores a value into the `AtomicPrimitive` if the current value is the
    /// same as the `current` value.
    ///
    /// The return value is a result indicating whether the new value was
    /// written and containing the previous value. On success this value is
    /// guaranteed to be equal to `current`.
    ///
    /// `compare_exchange` takes two `Ordering` arguments to describe the
    /// memory ordering of this operation. The first describes the required
    /// ordering if the operation succeeds while the second describes the
    /// required ordering when the operation fails. Using `Acquire` as success
    /// ordering makes the store part of this operation `Relaxed`, and using
    /// `Release` makes the successful load `Relaxed`. The failure ordering
    /// can only be `SeqCst`, `Acquire`, or `Relaxed` and must be equivalent
    /// to or weaker than the success ordering.
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;

    /// Stores a value into the `AtomicPrimitive` if the current value is the
    /// same as the `current` value.
    ///
    /// Unlike `compare_exchange`, this function is allowed to spuriously fail
    /// even when the comparison succeeds, which can result in more efficient
    /// code on some platforms. The return value is a result indicating whether
    /// the new value was written and containing the previous value.
    ///
    /// `compare_exchange_weak` takes two `Ordering` arguments to describe the
    /// memory ordering of this operation. The first describes the required
    /// ordering if the operation succeeds while the second describes the
    /// required ordering when the operation fails. Using `Acquire` as success
    /// ordering makes the store part of this operation `Relaxed`, and using
    /// `Release` makes the successful load `Relaxed`. The failure ordering
    /// can only be `SeqCst`, `Acquire`, or `Relaxed` and must be equivalent
    /// to or weaker than the success ordering.
    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;
}
