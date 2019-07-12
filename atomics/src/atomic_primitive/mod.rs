use crate::*;

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

pub trait AtomicPrimitive: Send + Sync {
    type Primitive;
    fn new(value: Self::Primitive) -> Self;
    fn get_mut(&mut self) -> &mut Self::Primitive;
    fn into_inner(self) -> Self::Primitive;
    fn load(&self, order: Ordering) -> Self::Primitive;
    fn store(&self, value: Self::Primitive, order: Ordering);
    fn swap(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn compare_and_swap(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        order: Ordering,
    ) -> Self::Primitive;
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;
    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;
}
