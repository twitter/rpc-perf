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

use datastructures::{Histogram, HistogramBuilder};
use libc::{c_float, uintptr_t};

/// Create a new `histogram`
#[no_mangle]
pub extern "C" fn histogram_new(
    min: uintptr_t,
    max: uintptr_t,
    precision: uintptr_t,
) -> *mut Histogram {
    Box::into_raw(HistogramBuilder::new(min, max, precision, None).build())
}

/// Clear the count stored in the `histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_clear(ptr: *mut Histogram) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.clear();
}

/// Get the count stored in the `histogram` for value
#[no_mangle]
pub unsafe extern "C" fn histogram_count(ptr: *mut Histogram, value: uintptr_t) -> uintptr_t {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.count(value)
}

/// Decrement the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_decr(ptr: *mut Histogram, value: uintptr_t, count: uintptr_t) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.decr(value, count);
}

/// Free the `Histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_free(ptr: *mut Histogram) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}

/// Increment the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_incr(ptr: *mut Histogram, value: uintptr_t, count: uintptr_t) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.incr(value, count);
}

/// Increment the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_percentile(
    ptr: *mut Histogram,
    percentile: c_float,
) -> uintptr_t {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.percentile(f64::from(percentile)).unwrap_or(0)
}

/// Get the total of all counts for the `histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_samples(ptr: *mut Histogram) -> uintptr_t {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.samples()
}

#[allow(dead_code)]
pub extern "C" fn fix_linking_when_not_using_stdlib() {
    panic!()
}

#[allow(dead_code)]
fn spare() {
    println!();
}
