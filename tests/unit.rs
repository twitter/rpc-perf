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

use std::process::Command;

#[cfg(test)]
#[test]
fn main() {
    test_subcrate("rpcperf_parser");
    test_subcrate("rpcperf_request");
    test_subcrate("rpcperf_workload");
}

fn test_subcrate(subcrate: &'static str) {
    let status = Command::new("cargo").args(&["test", "-p", subcrate]).status().unwrap();
    assert!(status.success(),
            "test for sub-crate: {} returned: {:?}",
            subcrate,
            status.code().unwrap());
}
