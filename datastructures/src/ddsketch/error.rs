// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Empty {}

/// Type of DDSketch error.
///
/// This enum is nonexhaustive and adding new variants
/// is not considered to be a breaking change.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DDSketchErrorKind {
    InvalidAlpha,
    TooManyBuckets,
    Unmergeable,

    // If we ever have more error conditions we can now
    // add them without worrying about breaking backwards
    // compatibility.
    #[doc(hidden)]
    __Nonexhaustive(Empty),
}

/// Error for when an operation with a DDSketch failed to complete.
#[derive(Debug)]
pub struct DDSketchError {
    kind: DDSketchErrorKind,
}

impl DDSketchError {
    pub(super) fn new(kind: DDSketchErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> DDSketchErrorKind {
        self.kind
    }
}

impl fmt::Display for DDSketchError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::DDSketchErrorKind::*;

        match self.kind {
            InvalidAlpha => write!(fmt, "Relative error bound outside the range (0, 1)"),
            TooManyBuckets => write!(fmt, "DDSketch would use too many buckets"),
            Unmergeable => write!(
                fmt,
                "Cannot merge sketches with different numbers of buckets"
            ),

            __Nonexhaustive(e) => match e {},
        }
    }
}

impl std::error::Error for DDSketchError {}
