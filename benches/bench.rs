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

#![cfg_attr(feature = "unstable", feature(test))]

#[cfg(feature = "unstable")]
extern crate test;

#[allow(unused_imports)]
use std::process;

use std::process::Command;

#[cfg(feature = "unstable")]
#[bench]
#[allow(unused_variables)]
fn main(b: &mut test::Bencher) {
    bench_subcrate("echo");
    bench_subcrate("memcache");
    bench_subcrate("ping");
    bench_subcrate("redis");
    bench_subcrate("thrift");
    process::exit(0);
}

#[allow(dead_code)]
fn bench_subcrate(subcrate: &str) {
    let prefix = "./lib/".to_owned();
    let path = prefix + subcrate;
    let status = Command::new("cargo")
        .args(&["bench", "--features", "unstable"])
        .current_dir(&path)
        .status()
        .unwrap();
    assert!(status.success(),
            "test for sub-crate: {} returned: {:?}",
            subcrate,
            status.code().unwrap());
}
