// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::ops::Deref;

use evmap::ShallowCopy;

use crate::{Counter, Gauge, Summary};

/// Analog to [`Cow`][stdcow] but for specific trait objects.
///
/// This allows for either storing a dynamic reference to a trait or a boxed
/// trait. However, it doesn't support most of the options that [`Cow`][stdcow]
/// supports since we are unable to promote a dyn trait reference to a boxed
/// trait.
///
/// [stdcow]: std::borrow::Cow
pub enum DynCow<'a, T: ?Sized> {
    /// A reference to a T
    Borrowed(&'a T),
    /// An owned box containing a T
    Owned(Box<T>),
}

impl<'a, T: ?Sized> DynCow<'a, T> {
    /// Create a `DynCow` from a pointer.
    ///
    /// # Safety
    /// For this to be safe `ptr` must be a pointer to a valid instance of a `T`
    /// and it must outlive the resulting `DynCow` instance.
    pub unsafe fn from_ptr(ptr: *const T) -> Self {
        Self::Borrowed(&*ptr)
    }

    /// Get a pointer pointing to the instance stored within this `DynCow`.
    pub fn as_ptr(&self) -> *const T {
        &**self as *const T
    }
}

impl<'a, T: ?Sized> Deref for DynCow<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::Borrowed(x) => x,
            Self::Owned(x) => &*x,
        }
    }
}

impl<'a, T: ?Sized> From<&'a T> for DynCow<'a, T> {
    fn from(val: &'a T) -> Self {
        Self::Borrowed(val)
    }
}

impl<'a, H> From<Box<H>> for DynCow<'a, dyn Summary + 'a>
where
    H: Summary + 'a,
{
    fn from(val: Box<H>) -> Self {
        Self::Owned(val)
    }
}

impl<'a, C> From<Box<C>> for DynCow<'a, dyn Counter + 'a>
where
    C: Counter + 'a,
{
    fn from(val: Box<C>) -> Self {
        Self::Owned(val)
    }
}

impl<'a, G> From<Box<G>> for DynCow<'a, dyn Gauge + 'a>
where
    G: Gauge + 'a,
{
    fn from(val: Box<G>) -> Self {
        Self::Owned(val)
    }
}

impl<'a, H> From<&'a H> for DynCow<'a, dyn Summary + 'a>
where
    H: Summary + 'a,
{
    fn from(val: &'a H) -> Self {
        Self::Borrowed(val)
    }
}

impl<'a, C> From<&'a C> for DynCow<'a, dyn Counter + 'a>
where
    C: Counter + 'a,
{
    fn from(val: &'a C) -> Self {
        Self::Borrowed(val)
    }
}

impl<'a, G> From<&'a G> for DynCow<'a, dyn Gauge + 'a>
where
    G: Gauge + 'a,
{
    fn from(val: &'a G) -> Self {
        Self::Borrowed(val)
    }
}

impl<'a, T: ?Sized> PartialEq for DynCow<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<'a, T: ?Sized> Eq for DynCow<'a, T> {}

impl<'a, T: ?Sized> ShallowCopy for DynCow<'a, T> {
    unsafe fn shallow_copy(&mut self) -> Self {
        match self {
            Self::Borrowed(x) => Self::Borrowed(x),
            Self::Owned(b) => Self::Owned(b.shallow_copy()),
        }
    }
}
