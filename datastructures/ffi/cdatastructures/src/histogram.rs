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

use datastructures::Histogram;
use libc::{c_float, uint32_t, uint64_t};

/// Create a new `histogram`
#[no_mangle]
pub extern "C" fn histogram_new(max: uint64_t, precision: uint32_t) -> *mut Histogram<u64> {
    Box::into_raw(Box::new(Histogram::new(max, precision, None, None)))
}

/// Clear the count stored in the `histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_clear(ptr: *mut Histogram<u64>) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.clear();
}

/// Decrement the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_decrement(
    ptr: *mut Histogram<u64>,
    value: uint64_t,
    count: uint64_t,
) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.decrement(value, count);
}

/// Free the `Histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_free(ptr: *mut Histogram<u64>) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}

/// Increment the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_increment(
    ptr: *mut Histogram<u64>,
    value: uint64_t,
    count: uint64_t,
) {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.increment(value, count);
}

/// Increment the value of the `Histogram` by count
#[no_mangle]
pub unsafe extern "C" fn histogram_percentile(
    ptr: *mut Histogram<u64>,
    percentile: c_float,
) -> uint64_t {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.percentile(f64::from(percentile)).unwrap_or(0)
}

/// Get the total of all counts for the `histogram`
#[no_mangle]
pub unsafe extern "C" fn histogram_total_count(ptr: *mut Histogram<u64>) -> uint64_t {
    let histogram = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    histogram.total_count()
}

#[allow(dead_code)]
pub extern "C" fn fix_linking_when_not_using_stdlib() {
    panic!()
}

#[allow(dead_code)]
fn spare() {
    println!();
}
