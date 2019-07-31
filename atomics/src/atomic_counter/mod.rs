use crate::atomic_primitive::AtomicPrimitive;
use core::sync::atomic::Ordering;

mod atomic_i16;
mod atomic_i32;
mod atomic_i64;
mod atomic_i8;
mod atomic_isize;
mod atomic_u16;
mod atomic_u32;
mod atomic_u64;
mod atomic_u8;
mod atomic_usize;

pub use self::atomic_i16::*;
pub use self::atomic_i32::*;
pub use self::atomic_i64::*;
pub use self::atomic_i8::*;
pub use self::atomic_isize::*;
pub use self::atomic_u16::*;
pub use self::atomic_u32::*;
pub use self::atomic_u64::*;
pub use self::atomic_u8::*;
pub use self::atomic_usize::*;

/// This trait is used to define the functions which are available on types
/// which may be used as atomic counters, allowing for them to be used as
/// generic types.
pub trait AtomicCounter: AtomicPrimitive
where
    Self::Primitive: Default + PartialEq + Copy,
{
    /// Adds to the current value, returning the previous value.
    ///
    /// This wraps around on overflow.
    ///
    /// `fetch_add` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_add(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Subtracts from the current value, returning the previous value.
    ///
    /// This wraps around on overflow.
    ///
    /// `fetch_sub` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_sub(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Bitwise "and" with the current value, returning the previous value.
    ///
    /// `fetch_and` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_and(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Bitwise "nand" with the current value, returning the previous value
    ///
    /// `fetch_nand` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_nand(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Bitwise "or" with the current value, returning the previous value
    ///
    /// `fetch_or` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_or(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Bitwise "xor" with the current value, returning the previous value
    ///
    /// `fetch_xor` take an `Ordering` argument which describes the memory
    /// ordering of the operation. All ordering modes are possible. Note that
    /// using `Acquire` makes the store part of this operation `Relaxed`, and
    /// using `Release` makes the load part of this operation `Relaxed`.
    fn fetch_xor(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;
}
