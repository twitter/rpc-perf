// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

/// A trait for all types that implementing saturating addition and subtraction
pub trait Saturating {
    fn saturating_add(&self, other: Self) -> Self;
    fn saturating_sub(&self, other: Self) -> Self;
}

impl Saturating for i8 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i8).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i8).saturating_sub(other)
    }
}

impl Saturating for i16 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i16).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i16).saturating_sub(other)
    }
}

impl Saturating for i32 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i32).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i32).saturating_sub(other)
    }
}

impl Saturating for i64 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as i64).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as i64).saturating_sub(other)
    }
}

impl Saturating for isize {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as isize).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as isize).saturating_sub(other)
    }
}

impl Saturating for u8 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u8).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u8).saturating_sub(other)
    }
}

impl Saturating for u16 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u16).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u16).saturating_sub(other)
    }
}

impl Saturating for u32 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u32).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u32).saturating_sub(other)
    }
}

impl Saturating for u64 {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as u64).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as u64).saturating_sub(other)
    }
}

impl Saturating for usize {
    fn saturating_add(&self, other: Self) -> Self {
        (*self as usize).saturating_add(other)
    }
    fn saturating_sub(&self, other: Self) -> Self {
        (*self as usize).saturating_sub(other)
    }
}
