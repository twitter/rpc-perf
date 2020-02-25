use core::cell::UnsafeCell;
pub use core::sync::atomic::Ordering;

mod traits;
pub use traits::*;

mod bool;
pub use crate::bool::*;

mod i8;
pub use crate::i8::*;

mod i16;
pub use crate::i16::*;

mod i32;
pub use crate::i32::*;

mod i64;
pub use crate::i64::*;

mod isize;
pub use crate::isize::*;

mod ptr;
pub use crate::ptr::*;

mod u8;
pub use crate::u8::*;

mod u16;
pub use crate::u16::*;

mod u32;
pub use crate::u32::*;

mod u64;
pub use crate::u64::*;

mod usize;
pub use crate::usize::*;

pub struct Atomic<T> {
    inner: UnsafeCell<T>,
}

impl<T> Atomic<T> {
    fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}
