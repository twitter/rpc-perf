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

pub trait AtomicCounter: AtomicPrimitive {
    /// Add to the current value and returns the previous value
    /// This wraps around on overflow
    fn fetch_add(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Subtract from the current value and returns the previous value
    /// This wraps around on overflow
    fn fetch_sub(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// Bitwise "and" with the current value and returns the previous value
    fn fetch_and(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    fn fetch_nand(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    fn fetch_or(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;

    fn fetch_xor(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;
}
