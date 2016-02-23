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

pub use super::{Parse, ParsedResponse};

use byteorder::{ByteOrder, BigEndian};

pub struct Response<'a> {
    pub response: &'a [u8],
}

impl<'a> Parse for Response<'a> {
    fn parse(&self) -> ParsedResponse {
        let bytes = self.response.len();
        if bytes > 4 {
            let length = BigEndian::read_u32(&self.response[0..4]);

            if bytes as u32 == (length + 4_u32) {
                return ParsedResponse::Ok;
            }
        }
        ParsedResponse::Incomplete
    }
}

#[cfg(test)]
mod tests {
    use super::{Parse, ParsedResponse, Response};

    #[test]
    fn test_parse_ok() {
        assert_eq!(Response { response: &[0, 0, 0, 1, 0] }.parse(), ParsedResponse::Ok);
        assert_eq!(Response { response: &[0, 0, 0, 2, 0, 1] }.parse(), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_incomplete() {
        assert_eq!(Response { response: &[0, 0] }.parse(), ParsedResponse::Incomplete);
        assert_eq!(Response { response: &[0, 0, 0, 1] }.parse(), ParsedResponse::Incomplete);
        assert_eq!(Response { response: &[0, 0, 0, 2, 0] }.parse(), ParsedResponse::Incomplete);
    }
}
