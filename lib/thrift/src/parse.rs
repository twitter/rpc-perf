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

use cfgtypes::ParsedResponse;

use common::bytes::{ByteOrder, BigEndian};

pub fn parse_response(response: &[u8]) -> ParsedResponse {
    let bytes = response.len();
    if bytes > 4 {
        let length = BigEndian::read_u32(&response[0..4]);

        if bytes as u32 == (length + 4_u32) {
            return ParsedResponse::Ok;
        }
    }
    ParsedResponse::Incomplete
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "unstable")]
    extern crate test;

    use super::parse_response;
    use cfgtypes::ParsedResponse;

    #[test]
    fn test_parse_ok() {
        assert_eq!(parse_response(&[0, 0, 0, 1, 0]), ParsedResponse::Ok);
        assert_eq!(parse_response(&[0, 0, 0, 2, 0, 1]), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_incomplete() {
        assert_eq!(parse_response(&[0, 0]), ParsedResponse::Incomplete);
        assert_eq!(parse_response(&[0, 0, 0, 1]), ParsedResponse::Incomplete);
        assert_eq!(parse_response(&[0, 0, 0, 2, 0]), ParsedResponse::Incomplete);
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_ok_benchmark(b: &mut test::Bencher) {
        let r = &[0, 0, 0, 1, 0];
        b.iter(|| parse_response(r));
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_incomplete_benchmark(b: &mut test::Bencher) {
        let r = &[0, 0, 0, 2, 0];
        b.iter(|| parse_response(r));
    }
}
