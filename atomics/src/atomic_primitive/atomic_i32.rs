// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::{AtomicPrimitive, Ordering};

#[cfg(feature = "serde")]
use serde::{de::Deserialize, de::Visitor, Deserializer};

/// An integer type which can be safely shared between threads.
pub struct AtomicI32 {
    pub(crate) inner: core::sync::atomic::AtomicI32,
}

impl AtomicPrimitive for AtomicI32 {
    type Primitive = i32;

    fn new(value: Self::Primitive) -> Self {
        Self {
            inner: core::sync::atomic::AtomicI32::new(value),
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

impl Default for AtomicI32 {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl PartialEq for AtomicI32 {
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl Eq for AtomicI32 {}

impl std::fmt::Debug for AtomicI32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

#[cfg(feature = "serde")]
struct AtomicI32Visitor;

#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for AtomicI32Visitor {
    type Value = AtomicI32;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a signed 32bit integer")
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(i32::from(value)))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(i32::from(value)))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(i32::from(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use std::convert::TryFrom;
        if let Ok(value) = i32::try_from(value) {
            Ok(Self::Value::new(value))
        } else {
            Err(E::custom(format!("i32 is out of range: {}", value)))
        }
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(i32::from(value)))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::new(i32::from(value)))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use std::convert::TryFrom;
        if let Ok(value) = i32::try_from(value) {
            Ok(Self::Value::new(value))
        } else {
            Err(E::custom(format!("i32 is out of range: {}", value)))
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use std::convert::TryFrom;
        if let Ok(value) = i32::try_from(value) {
            Ok(Self::Value::new(value))
        } else {
            Err(E::custom(format!("i32 is out of range: {}", value)))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AtomicI32 {
    fn deserialize<D>(deserializer: D) -> Result<AtomicI32, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i32(AtomicI32Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load() {
        let atomic = AtomicI32::new(0);
        assert_eq!(atomic.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn store() {
        let atomic = AtomicI32::new(0);
        atomic.store(1, Ordering::SeqCst);
        assert_eq!(atomic.into_inner(), 1);
    }

    #[test]
    fn swap() {
        let atomic = AtomicI32::new(0);
        assert_eq!(atomic.swap(1, Ordering::SeqCst), 0);
    }

    #[test]
    fn compare_and_swap() {
        let atomic = AtomicI32::new(0);
        assert_eq!(atomic.compare_and_swap(0, 1, Ordering::SeqCst), 0);
        assert_eq!(atomic.compare_and_swap(0, 2, Ordering::SeqCst), 1);
    }

    #[test]
    fn compare_exchange() {
        let atomic = AtomicI32::new(0);
        assert_eq!(
            atomic.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst),
            Ok(0)
        );
        assert_eq!(
            atomic.compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst),
            Err(1)
        );
    }

    #[test]
    fn compare_exchange_weak() {
        let atomic = AtomicI32::new(0);
        loop {
            if atomic
                .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
        assert_eq!(atomic.into_inner(), 1);
    }
}
