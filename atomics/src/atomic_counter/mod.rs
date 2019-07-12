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

pub trait AtomicCounter: AtomicPrimitive
where
    Self::Primitive: Default + PartialEq + Copy,
{
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

    // /// Saturating add using atomic intrinsics
    // fn saturating_add(&self, value: Self::Primitive) -> Self::Primitive {
    //     let mut current = self.load(Ordering::SeqCst);
    //     let mut new = current.saturating_add(value);
    //     loop {
    //         let result = self.compare_and_swap(current, new, Ordering::SeqCst);
    //         if result == current {
    //             return current;
    //         }
    //         new = result.saturating_add(value);
    //         current = result;
    //     }
    // }

    // /// Saturating sub using atomic intrinsics
    // fn saturating_sub(&self, value: Self::Primitive) -> Self::Primitive {
    //     let mut current = self.load(Ordering::SeqCst);
    //     let mut new = current.saturating_sub(value);
    //     loop {
    //         let result = self.compare_and_swap(current, new, Ordering::SeqCst);
    //         if result == current {
    //             return current;
    //         }
    //         new = result.saturating_sub(value);
    //         current = result;
    //     }
    // }
}

// pub trait Saturating {
//     fn saturating_add(&self, rhs: Self) -> Self;
//     fn saturating_sub(&self, rhs: Self) -> Self;
// }

// impl Saturating for i8 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as i8).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as i8).saturating_sub(rhs)
//     }
// }
// impl Saturating for i16 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as i16).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as i16).saturating_sub(rhs)
//     }
// }
// impl Saturating for i32 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as i32).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as i32).saturating_sub(rhs)
//     }
// }
// impl Saturating for i64 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as i64).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as i64).saturating_sub(rhs)
//     }
// }
// impl Saturating for isize {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as isize).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as isize).saturating_sub(rhs)
//     }
// }
// impl Saturating for u8 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as u8).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as u8).saturating_sub(rhs)
//     }
// }
// impl Saturating for u16 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as u16).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as u16).saturating_sub(rhs)
//     }
// }
// impl Saturating for u32 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as u32).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as u32).saturating_sub(rhs)
//     }
// }
// impl Saturating for u64 {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as u64).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as u64).saturating_sub(rhs)
//     }
// }
// impl Saturating for usize {
//     fn saturating_add(&self, rhs: Self) -> Self {
//         (*self as usize).saturating_add(rhs)
//     }
//     fn saturating_sub(&self, rhs: Self) -> Self {
//         (*self as usize).saturating_sub(rhs)
//     }
// }
