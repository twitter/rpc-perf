// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

/// A value that can be recorded into a metric.
///
/// You shouldn't have to deal with this in most code. It is used by this
/// library to accept a wider variety of types.
///
/// However, if you want to make your type something that can be natively
/// accepted by this metrics library, implement `From` or `Into<MetricsValue>`
/// for your type.
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug)]
pub enum MetricValue {
    Signed(i64),
    Unsigned(u64),
}

impl MetricValue {
    /// Get this value as a `u64` if it can be converted losslessly.
    #[inline]
    pub fn as_u64(self) -> Option<u64> {
        match self {
            Self::Signed(x) if x >= 0 => Some(x as u64),
            Self::Signed(_) => None,
            Self::Unsigned(x) => Some(x),
        }
    }

    /// Get this value as a `i64` if it can be converted losslessly.
    #[inline]
    pub fn as_i64(self) -> Option<i64> {
        match self {
            Self::Unsigned(x) if x > std::i64::MAX as u64 => None,
            Self::Unsigned(x) => Some(x as i64),
            Self::Signed(x) => Some(x),
        }
    }

    /// Convert this value to a `u64`.
    #[inline]
    pub fn as_u64_unchecked(self) -> u64 {
        match self {
            Self::Signed(x) => x as u64,
            Self::Unsigned(x) => x,
        }
    }

    /// Convert this value to an `i64`.
    #[inline]
    pub fn as_i64_unchecked(self) -> i64 {
        match self {
            Self::Signed(x) => x,
            Self::Unsigned(x) => x as i64,
        }
    }
}

impl From<u8> for MetricValue {
    fn from(v: u8) -> Self {
        Self::Unsigned(v.into())
    }
}

impl From<u16> for MetricValue {
    fn from(v: u16) -> Self {
        Self::Unsigned(v.into())
    }
}

impl From<u32> for MetricValue {
    fn from(v: u32) -> Self {
        Self::Unsigned(v.into())
    }
}

impl From<u64> for MetricValue {
    fn from(v: u64) -> Self {
        Self::Unsigned(v)
    }
}

impl From<i8> for MetricValue {
    fn from(v: i8) -> Self {
        Self::Signed(v.into())
    }
}

impl From<i16> for MetricValue {
    fn from(v: i16) -> Self {
        Self::Signed(v.into())
    }
}

impl From<i32> for MetricValue {
    fn from(v: i32) -> Self {
        Self::Signed(v.into())
    }
}

impl From<i64> for MetricValue {
    fn from(v: i64) -> Self {
        Self::Signed(v)
    }
}
