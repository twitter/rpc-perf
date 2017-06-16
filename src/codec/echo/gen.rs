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

use crc;
use std::mem::transmute;

/// create an echo request with given value
pub fn echo(value: &[u8]) -> Vec<u8> {
    let crc = crc::crc32::checksum_ieee(value);
    let mut msg: Vec<u8> = Vec::new();
    msg.extend(value.iter().cloned());
    let bytes: [u8; 4] = unsafe { transmute(crc.to_be()) };
    msg.extend(bytes.iter().cloned());
    msg.extend([13, 10].iter().cloned());
    msg
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[cfg(feature = "unstable")]
    #[allow(unused_imports)]
    use test;

    #[cfg(feature = "unstable")]
    #[bench]
    fn echo_benchmark(b: &mut test::Bencher) {
        b.iter(|| echo(b"123456789"));
    }

    #[test]
    fn echo_test() {
        assert_eq!(
            echo(b"123456789"),
            [49, 50, 51, 52, 53, 54, 55, 56, 57, 203, 244, 57, 38, 13, 10]
        );
    }
}
