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

pub struct Response<'a> {
    pub response: &'a str,
}

impl<'a> Parse for Response<'a> {
    fn parse(&self) -> ParsedResponse {

        let mut lines: Vec<&str> = self.response.split("\r\n").collect();

        // expect an empty line from the split
        if lines[lines.len() - 1] == "" {
            let _ = lines.pop();
        } else {
            return ParsedResponse::Incomplete;
        }

        let tokens: Vec<&str> = lines[0].split_whitespace().collect();

        // one line responses can be special cased
        if lines.len() == 1 {
            // complete responses are short exactly 2 bytes for CRLF
            if lines[0].len() + 2 != self.response.len() {
                return ParsedResponse::Incomplete;
            }

            // special case 1 token responses
            if tokens.len() == 1 {
                match &*tokens[0] {
                    "PONG" => {
                        return ParsedResponse::Ok;
                    }
                    "+PONG" => {
                        return ParsedResponse::Ok;
                    }
                    _ => {}
                }
            }
        }
        ParsedResponse::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{Parse, ParsedResponse, Response};

    #[test]
    fn test_parse_pong() {
        let r = Response { response: "PONG" };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);

        let r = Response { response: "PONG\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Ok);

        let r = Response { response: "ERROR\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Unknown);
    }
}
