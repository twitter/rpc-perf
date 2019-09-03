// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;

#[cfg(feature = "serde")]
use serde::{de::Error, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use core::marker::PhantomData;

pub struct AtomicOption<T>
where
    T: AtomicPrimitive + Default,
{
    inner: T,
    is_some: AtomicBool,
}

impl<T> AtomicOption<T>
where
    T: AtomicPrimitive + Default,
{
    pub fn some(value: T) -> Self {
        Self {
            inner: value,
            is_some: AtomicBool::new(true),
        }
    }

    pub fn none() -> Self {
        Self {
            inner: T::default(),
            is_some: AtomicBool::new(false),
        }
    }

    pub fn take(&self, ordering: Ordering) -> Option<<T as AtomicPrimitive>::Primitive> {
        if self
            .is_some
            .compare_exchange(true, false, ordering, ordering)
            .is_ok()
        {
            Some(self.inner.load(ordering))
        } else {
            None
        }
    }

    pub fn replace(
        &self,
        new: Option<<T as AtomicPrimitive>::Primitive>,
        ordering: Ordering,
    ) -> Option<<T as AtomicPrimitive>::Primitive> {
        match new {
            Some(value) => {
                if self.is_some(ordering) {
                    Some(self.inner.swap(value, ordering))
                } else {
                    self.inner.store(value, ordering);
                    self.is_some.store(true, ordering);
                    None
                }
            }
            None => self.take(ordering),
        }
    }

    pub fn is_some(&self, ordering: Ordering) -> bool {
        self.is_some.load(ordering)
    }

    pub fn is_none(&self, ordering: Ordering) -> bool {
        !self.is_some(ordering)
    }

    pub fn unwrap_or(
        &self,
        def: <T as AtomicPrimitive>::Primitive,
        ordering: Ordering,
    ) -> <T as AtomicPrimitive>::Primitive {
        if self.is_some(ordering) {
            self.inner.load(ordering)
        } else {
            def
        }
    }

    pub fn unwrap_or_else<F>(&self, f: F, ordering: Ordering) -> <T as AtomicPrimitive>::Primitive
    where
        F: FnOnce() -> <T as AtomicPrimitive>::Primitive,
    {
        if self.is_some(ordering) {
            self.inner.load(ordering)
        } else {
            f()
        }
    }

    pub fn map<U, F>(&self, f: F, ordering: Ordering) -> Option<U>
    where
        F: FnOnce(<T as AtomicPrimitive>::Primitive) -> U,
    {
        if self.is_some(ordering) {
            Some(f(self.inner.load(ordering)))
        } else {
            None
        }
    }

    pub fn map_or<U, F>(&self, def: U, f: F, ordering: Ordering) -> U
    where
        F: FnOnce(<T as AtomicPrimitive>::Primitive) -> U,
    {
        if self.is_some(ordering) {
            f(self.inner.load(ordering))
        } else {
            def
        }
    }

    pub fn load(&self, ordering: Ordering) -> Option<<T as AtomicPrimitive>::Primitive> {
        if self.is_some(ordering) {
            Some(self.inner.load(ordering))
        } else {
            None
        }
    }
}

impl<T> Default for AtomicOption<T>
where
    T: AtomicPrimitive + Default,
{
    fn default() -> Self {
        Self::none()
    }
}

impl<T> PartialEq for AtomicOption<T>
where
    T: AtomicPrimitive + Default,
{
    fn eq(&self, other: &Self) -> bool {
        if self.is_some.load(Ordering::SeqCst) {
            if other.is_some.load(Ordering::SeqCst) {
                self.inner == other.inner
            } else {
                false
            }
        } else {
            if other.is_some.load(Ordering::SeqCst) {
                false
            } else {
                true
            }
        }
    }
}

impl<T> Eq for AtomicOption<T> where T: AtomicPrimitive + Default {}

impl<T> std::fmt::Debug for AtomicOption<T>
where
    T: AtomicPrimitive + Default,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

#[cfg(feature = "serde")]
struct AtomicOptionVisitor<T> {
    marker: PhantomData<T>,
}

#[cfg(feature = "serde")]
impl<'de, T> Visitor<'de> for AtomicOptionVisitor<T>
where
    T: Deserialize<'de> + AtomicPrimitive + Default,
{
    type Value = AtomicOption<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a boolean value")
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(AtomicOption::none())
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(AtomicOption::none())
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| AtomicOption::some(v))
    }
}

#[cfg(feature = "serde")]
impl<'de, T> Deserialize<'de> for AtomicOption<T>
where
    T: Deserialize<'de> + Default + AtomicPrimitive,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(AtomicOptionVisitor {
            marker: PhantomData,
        })
    }
}

#[cfg(feature = "serde")]
impl<T> Serialize for AtomicOption<T>
where
    T: Serialize + AtomicPrimitive + Default,
    <T as AtomicPrimitive>::Primitive: Serialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let option = self.load(Ordering::SeqCst);
        match option {
            Some(ref v) => serializer.serialize_some(v),
            None => serializer.serialize_none(),
        }
    }
}
