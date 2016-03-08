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

extern crate crc;

pub use super::{Parse, ParsedResponse};

pub use crc::crc32;

use std::mem::transmute;

pub struct Response<'a> {
    pub response: &'a [u8],
}

impl<'a> Parse for Response<'a> {
    fn parse(&self) -> ParsedResponse {

        if self.response.len() <= 6 {
            return ParsedResponse::Incomplete;
        }

        let (msg, crlf) = self.response.split_at((self.response.len() - 2));

        if crlf != [13, 10] {
            return ParsedResponse::Incomplete;
        }

        let (value, crc) = msg.split_at((msg.len() - 4));
        assert!(crc.len() == 4);
        let crc_calc = crc::crc32::checksum_ieee(value);
        let crc_bytes: [u8; 4] = unsafe { transmute(crc_calc.to_be()) };
        if crc == crc_bytes {
            return ParsedResponse::Ok;
        } else {
            debug!("CRC RECV: {:?} CRC CALC: {:?}", crc, crc_bytes);
        }
        ParsedResponse::Error("bad crc".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{Parse, ParsedResponse, Response};

    #[test]
    fn test_parse_incomplete() {
        let r = Response { response: &[0] };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);

        let r = Response { response: &[0, 1, 2, 3, 4, 5, 6] };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);

        let r = Response { response: &[0, 1, 2, 3, 4, 5, 6, 13] };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_ok() {
        let r = Response { response: &[0, 1, 2, 8, 84, 137, 127, 13, 10] };
        assert_eq!(r.parse(), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = Response { response: "3421780262\r\n".as_bytes() };
        assert_eq!(r.parse(), ParsedResponse::Error("bad crc".to_string()));
    }
}
