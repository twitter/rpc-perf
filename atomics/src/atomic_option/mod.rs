// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;

#[cfg(feature = "serde")]
use serde::{de::Error, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use core::marker::PhantomData;

/// An implementation of `Option` which is thread safe when it is used to hold
/// types implementing `AtomicPrimitive`.
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
    /// Creates a new `AtomicOption` containing some value.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::some(AtomicUsize::new(1));
    /// ```
    pub fn some(value: T) -> Self {
        Self {
            inner: value,
            is_some: AtomicBool::new(true),
        }
    }

    /// Creates a new `AtomicOption` which does not contain a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// ```
    pub fn none() -> Self {
        Self {
            inner: T::default(),
            is_some: AtomicBool::new(false),
        }
    }

    /// Takes the inner value out of the `AtomicOption` as its Primitive type
    /// leaving a `None` in its place.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::some(AtomicUsize::new(1));
    /// let y = x.take(Ordering::SeqCst);
    /// assert_eq!(y, Some(1));
    /// assert!(x.is_none(Ordering::SeqCst));
    /// ```
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

    /// Replaces the inner value in the option with the value provided. Returns
    /// the previous value if present.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::some(AtomicUsize::new(1));
    /// let y = x.replace(Some(2), Ordering::SeqCst);
    /// assert_eq!(y, Some(1));
    /// assert!(x.is_some(Ordering::SeqCst));
    /// assert_eq!(x.load(Ordering::SeqCst), Some(2));
    /// ```
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

    /// Returns true if the option contains a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::some(AtomicUsize::new(1));
    /// assert!(x.is_some(Ordering::SeqCst));
    /// ```
    pub fn is_some(&self, ordering: Ordering) -> bool {
        self.is_some.load(ordering)
    }

    /// Returns true if the option does not contain a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// assert!(x.is_none(Ordering::SeqCst));
    /// ```
    pub fn is_none(&self, ordering: Ordering) -> bool {
        !self.is_some(ordering)
    }

    /// Returns the contained value or a default.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// assert_eq!(x.unwrap_or(1, Ordering::SeqCst), 1);
    ///
    /// let y = AtomicOption::some(AtomicUsize::new(2));
    /// assert_eq!(y.unwrap_or(1, Ordering::SeqCst), 2);
    /// ```
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

    /// Returns the contained value or computes it from a closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// assert_eq!(x.unwrap_or_else(|| { 1 + 1 }, Ordering::SeqCst), 2);
    ///
    /// let y = AtomicOption::some(AtomicUsize::new(2));
    /// assert_eq!(y.unwrap_or_else(|| { 2 + 1 }, Ordering::SeqCst), 2);
    /// ```
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

    /// Maps an `AtomicOption<T>` to an `AtomicOption<U>` by applying a function
    /// to the contained value.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// assert_eq!(x.map(|v| { v + 1}, Ordering::SeqCst), None);
    ///
    /// let y = AtomicOption::some(AtomicUsize::new(1));
    /// assert_eq!(y.map(|v| { v + 1}, Ordering::SeqCst), Some(2));
    /// ```
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

    /// Applies a function to the contained value (if any), or returns the
    /// provided default (if not).
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::<AtomicUsize>::none();
    /// assert_eq!(x.map_or(1, |v| { v + 1}, Ordering::SeqCst), 1);
    ///
    /// let y = AtomicOption::some(AtomicUsize::new(1));
    /// assert_eq!(y.map_or(1, |v| { v + 1}, Ordering::SeqCst), 2);
    /// ```
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

    /// Converts the `AtomicOption<T>` to an `Option` of the primitive type.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomics::*;
    ///
    /// let x = AtomicOption::some(AtomicUsize::new(1));
    /// assert_eq!(x.load(Ordering::SeqCst), Some(1));
    ///
    /// let y = AtomicOption::<AtomicUsize>::none();
    /// assert_eq!(y.load(Ordering::SeqCst), None);
    /// ```
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
