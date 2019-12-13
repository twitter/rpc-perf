// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! Different implementations of DDSketch.

mod atomic;
mod dense;
mod error;

pub use self::atomic::AtomicDDSketch;
pub use self::dense::DenseDDSketch;
pub use self::error::{DDSketchError, DDSketchErrorKind};
