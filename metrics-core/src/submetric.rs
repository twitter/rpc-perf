// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::borrow::Cow;

/// A value returned from a submetric.
///
/// This is meant to allow submetrics to return any value
/// that can reasonably be supported in downstream systems.
#[derive(Copy, Clone, Debug)]
pub enum SubMetricValue {
    /// An unsigned integer
    Unsigned(u64),
    /// A signed integer
    Signed(i64),
    /// A floating point number
    Float(f64),
}

/// A bucket within a histogram.
///
/// It has minimum and maximum bounds on the bucket as well
/// as the number of samples stored within the bucket.
#[derive(Copy, Clone, Debug)]
pub struct Bucket {
    /// The lower bound of the bucket
    pub min: u64,
    /// The upper bound of the bucket
    pub max: u64,
    /// The number of samples within the bucket.
    pub count: u64,
}

/// An arbitrary stat returned by a summary.
#[derive(Clone, Debug)]
pub struct SubMetric {
    /// The name of the submetric
    pub name: Cow<'static, str>,
    /// The value of the submetric
    pub value: SubMetricValue,
}

impl SubMetric {
    /// Create a new submetric with a name and value.
    pub fn new(name: impl Into<Cow<'static, str>>, value: impl Into<SubMetricValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

macro_rules! decl_from_submetricvalue {
    {
        $( $ty:ty => $class:ident; )*
    } => {
        $(
            impl From<$ty> for SubMetricValue {
                fn from(v: $ty) -> Self {
                    Self::$class(v.into())
                }
            }
        )*
    }
}

decl_from_submetricvalue! {
    u64 => Unsigned;
    u32 => Unsigned;
    u16 => Unsigned;
    u8  => Unsigned;

    i64 => Signed;
    i32 => Signed;
    i16 => Signed;
    i8  => Signed;

    f32 => Float;
    f64 => Float;
}
