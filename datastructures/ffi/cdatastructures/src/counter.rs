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

use datastructures::Counter;
use libc::uint64_t;

/// Create a new `Counter`
#[no_mangle]
pub extern "C" fn counter_new() -> *mut Counter<u64> {
    Box::into_raw(Box::new(Counter::<u64>::default()))
}

/// Clear the count stored in the `Counter`
#[no_mangle]
pub unsafe extern "C" fn counter_reset(ptr: *mut Counter<u64>) {
    let counter = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    counter.set(0);
}

/// Get the count stored in the `Counter`
#[no_mangle]
pub unsafe extern "C" fn counter_count(ptr: *mut Counter<u64>) -> uint64_t {
    let counter = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    counter.get()
}

/// Decrement the value of the `Counter` by count
#[no_mangle]
pub unsafe extern "C" fn counter_sub(ptr: *mut Counter<u64>, count: uint64_t) {
    let counter = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    counter.sub(count);
}

/// Free the `Counter`
#[no_mangle]
pub unsafe extern "C" fn counter_free(ptr: *mut Counter<u64>) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}

/// Increment the value of the `Counter` by count
#[no_mangle]
pub unsafe extern "C" fn counter_add(ptr: *mut Counter<u64>, count: uint64_t) {
    let counter = {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    counter.add(count);
}

#[allow(dead_code)]
pub extern "C" fn fix_linking_when_not_using_stdlib() {
    panic!()
}
