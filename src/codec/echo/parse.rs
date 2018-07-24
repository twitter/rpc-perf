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
use crc;

use std::mem::transmute;

pub fn parse_response(response: &[u8]) -> ParsedResponse {
    if response.len() <= 6 {
        return ParsedResponse::Incomplete;
    }

    let (msg, crlf) = response.split_at(response.len() - 2);

    if crlf != [13, 10] {
        return ParsedResponse::Incomplete;
    }

    let (value, crc) = msg.split_at(msg.len() - 4);

    if !(crc.len() == 4) {
        return ParsedResponse::Unknown;
    }

    let crc_calc = crc::crc32::checksum_ieee(value);
    let crc_bytes: [u8; 4] = unsafe { transmute(crc_calc.to_be()) };
    if crc == crc_bytes {
        return ParsedResponse::Ok;
    } else {
        debug!("CRC RECV: {:?} CRC CALC: {:?}", crc, crc_bytes);
    }
    ParsedResponse::Error("bad crc".to_owned())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "unstable")]
    extern crate test;

    use super::parse_response;
    use cfgtypes::ParsedResponse;

    #[test]
    fn test_parse_incomplete() {
        let r = [0];
        assert_eq!(parse_response(&r), ParsedResponse::Incomplete);

        let r = [0, 1, 2, 3, 4, 5, 6];
        assert_eq!(parse_response(&r), ParsedResponse::Incomplete);

        let r = [0, 1, 2, 3, 4, 5, 6, 13];
        assert_eq!(parse_response(&r), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_ok() {
        let r = [0, 1, 2, 8, 84, 137, 127, 13, 10];
        assert_eq!(parse_response(&r), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = "3421780262\r\n".as_bytes();
        assert_eq!(
            parse_response(&r),
            ParsedResponse::Error("bad crc".to_owned())
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_ok_benchmark(b: &mut test::Bencher) {
        let r = &[0, 1, 2, 8, 84, 137, 127, 13, 10];
        b.iter(|| parse_response(r));
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_err_benchmark(b: &mut test::Bencher) {
        let r = "3421780262\r\n".as_bytes();
        b.iter(|| parse_response(r));
    }
}
