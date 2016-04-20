//  rpc-perf - RPC Performance Testing
//  Copyright 2015 Twitter, Inc
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

use super::gen;
use super::*;
use cfgtypes::Style;

#[cfg(feature = "unstable")]
extern crate test;

pub fn mk_param(id: i16, value: Tvalue) -> Parameter {
    Parameter {
        id: Some(id),
        seed: 0,
        size: 1,
        style: Style::Static,
        regenerate: false,
        value: value,
    }
}

// thrift calculator `add` example
#[test]
fn thrift_add() {
    let mut payload = Vec::new();
    payload.push(mk_param(1, Tvalue::Int32(1)));
    payload.push(mk_param(2, Tvalue::Int32(1)));

    assert_eq!(gen::generic("add", 0, &mut payload),
               vec![0, 0, 0, 30, 128, 1, 0, 1, 0, 0, 0, 3, 97, 100, 100, 0, 0, 0, 0, 8, 0, 1,
                    0, 0, 0, 1, 8, 0, 2, 0, 0, 0, 1, 0]);
}

// thrift calculator subtraction example
#[test]
fn thrift_subtract() {
    let mut payload = Vec::new();
    payload.push(mk_param(1, Tvalue::Int32(1)));
    payload.push(mk_param(2, Tvalue::Struct));
    payload.push(mk_param(1, Tvalue::Int32(15)));
    payload.push(mk_param(2, Tvalue::Int32(10)));
    payload.push(mk_param(3, Tvalue::Int32(2)));
    payload.push(mk_param(0, Tvalue::Stop));

    assert_eq!(gen::generic("calculate", 0, &mut payload),
               vec![0, 0, 0, 54, 128, 1, 0, 1, 0, 0, 0, 9, 99, 97, 108, 99, 117, 108, 97,
                    116, 101, 0, 0, 0, 0, 8, 0, 1, 0, 0, 0, 1, 12, 0, 2, 8, 0, 1, 0, 0, 0,
                    15, 8, 0, 2, 0, 0, 0, 10, 8, 0, 3, 0, 0, 0, 2, 0, 0]);
}
