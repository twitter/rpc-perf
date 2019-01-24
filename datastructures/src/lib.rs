//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! A collection of thread-safe datastructures which are intended for
//! inclusion in other common libraries or for use directly in binary
//! projects. Additionally, these datastructures can be called from other
//! languages.
//!
//! # Goals
//! * provide a set of useful datastructures
//! * datastructures should be thread-safe
//! * useful from other languages via FFI
//!
//! # Overview
//!
//! ## Counter
//! An atomic counter which is thread-safe. This is a very fast counter
//! which implements all operations using atomic primitives. All atomic
//! operations with exception of `Store` (which is used for `set()`) are
//! using `Relaxed` ordering. `set()` is performed using `Sequential
//! Consistent` ordering. Approximate speed is on the order of tens of
//! millions of increments per second on a developer laptop.
//!
//! # FixedHistogram
//! A thread-safe fixed-size histogram. It utilizes logarithimic outer
//! buckets with linear inner buckets to maintain precision across the full
//! range of stored values. This datastructure may be used to aggregate
//! across all samples, or latched externally to produce percentiles across
//! a time-range. Approximate speed is on the order of tens of millions of
//! increments per second on a developer laptop.
//!
//! ## ManagedHistogram
//! A thread-safe version of a `FixedHistogram` which can be resized at
//! runtime. Resizing will cause all existing samples to be lost, but can
//! allow for cases where the precision or range must be changed at runtime.
//! This datastructure is significantly slower than `FixedHistogram`.
//! Approximate speed is on the order of millions of increments per second
//! on a developer laptop.
//!
//! ## MovingHistogram
//! A thread-safe histogram which retains values within a set `Duration`.
//! Older samples are automatically aged-out. This `Histogram` type can be
//! used to produce percentiles that are representative of the window
//! specified with the `Duration`. This type can produce its percentiles at
//! any time. This comes at a significant performance cost. Approximate
//! speed is on the order of hundreds of thousands of increments per second
//! on a developer laptop.
//!
//! ## RwWrapper
//! This type can be used to provide blocking writes or non-blocking writes
//! to the inner datastructure. This would typically be used with types that
//! have both thread-safe and non-thread-safe actions.
//!
//! ## Wrapper
//! This type can be used to provide interior mutability for thread-safe
//! inner types. This should only be used for types that are fully
//! thread-safe.

#![allow(dead_code)]

mod bool;
mod counter;
pub mod heatmap;
pub mod histogram;
mod wrapper;

pub use crate::bool::Bool;
pub use crate::counter::Counter;
pub use crate::heatmap::{Config as HeatmapBuilder, Heatmap};
pub use crate::histogram::{Config as HistogramConfig, Histogram};
pub use crate::wrapper::{RwWrapper, Wrapper};
