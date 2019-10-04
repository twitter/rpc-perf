// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::{AtomicPrimitive, Ordering};

#[cfg(feature = "serde")]
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

/// A double precision floating point type which can be safely shared between
/// threads.
pub struct AtomicF64 {
    pub(crate) inner: core::sync::atomic::AtomicU64,
}

impl AtomicPrimitive for AtomicF64 {
    type Primitive = f64;

    fn new(value: Self::Primitive) -> Self {
        let value = unsafe { std::mem::transmute(value) };
        Self {
            inner: core::sync::atomic::AtomicU64::new(value),
        }
    }

    fn get_mut(&mut self) -> &mut Self::Primitive {
        unsafe { &mut *(self.inner.get_mut() as *mut u64 as *mut f64) }
    }

    fn into_inner(self) -> Self::Primitive {
        f64::from_bits(self.inner.into_inner())
    }

    fn load(&self, order: Ordering) -> Self::Primitive {
        f64::from_bits(self.inner.load(order))
    }

    fn store(&self, value: Self::Primitive, order: Ordering) {
        self.inner.store(value.to_bits(), order);
    }

    fn swap(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        f64::from_bits(self.inner.swap(value.to_bits(), order))
    }

    fn compare_and_swap(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        order: Ordering,
    ) -> Self::Primitive {
        f64::from_bits(
            self.inner
                .compare_and_swap(current.to_bits(), new.to_bits(), order),
        )
    }

    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.inner
            .compare_exchange(current.to_bits(), new.to_bits(), success, failure)
            .map(f64::from_bits)
            .map_err(f64::from_bits)
    }

    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive> {
        self.inner
            .compare_exchange_weak(current.to_bits(), new.to_bits(), success, failure)
            .map(f64::from_bits)
            .map_err(f64::from_bits)
    }
}

impl Default for AtomicF64 {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl PartialEq for AtomicF64 {
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl Eq for AtomicF64 {}

impl std::fmt::Debug for AtomicF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.load(Ordering::Relaxed))
    }
}

#[cfg(feature = "serde")]
struct AtomicF64Visitor;

#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for AtomicF64Visitor {
    type Value = AtomicF64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a 64bit floating point number")
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use std::convert::TryFrom;
        if let Ok(value) = i32::try_from(value).map(f64::from) {
            Ok(Self::Value::new(value))
        } else {
            Err(E::custom(format!("f64 is out of range: {}", value)))
        }
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use std::convert::TryFrom;
        if let Ok(value) = u32::try_from(value).map(f64::from) {
            Ok(Self::Value::new(value))
        } else {
            Err(E::custom(format!("f64 is out of range: {}", value)))
        }
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(f64::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(value))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AtomicF64 {
    fn deserialize<D>(deserializer: D) -> Result<AtomicF64, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(AtomicF64Visitor)
    }
}

#[cfg(feature = "serde")]
impl Serialize for AtomicF64 {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_some(&self.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load() {
        let atomic = AtomicF64::new(0.0);
        assert_eq!(atomic.load(Ordering::SeqCst), 0.0);
    }

    #[test]
    fn store() {
        let atomic = AtomicF64::new(0.0);
        atomic.store(3.14, Ordering::SeqCst);
        assert_eq!(atomic.into_inner(), 3.14);
    }

    #[test]
    fn swap() {
        let atomic = AtomicF64::new(0.0);
        assert_eq!(atomic.swap(3.14, Ordering::SeqCst), 0.0);
    }

    #[test]
    fn compare_and_swap() {
        let atomic = AtomicF64::new(0.0);
        assert_eq!(atomic.compare_and_swap(0.0, 3.14, Ordering::SeqCst), 0.0);
        assert_eq!(atomic.compare_and_swap(0.0, 42.0, Ordering::SeqCst), 3.14);
    }

    #[test]
    fn compare_exchange() {
        let atomic = AtomicF64::new(0.0);
        assert_eq!(
            atomic.compare_exchange(0.0, 3.14, Ordering::SeqCst, Ordering::SeqCst),
            Ok(0.0)
        );
        assert_eq!(
            atomic.compare_exchange(0.0, 42.0, Ordering::SeqCst, Ordering::SeqCst),
            Err(3.14)
        );
    }

    #[test]
    fn compare_exchange_weak() {
        let atomic = AtomicF64::new(0.0);
        loop {
            if atomic
                .compare_exchange(0.0, 3.14, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
        assert_eq!(atomic.into_inner(), 3.14);
    }
}
