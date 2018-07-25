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

pub fn parse_response(response: &str) -> ParsedResponse {
    let mut lines: Vec<&str> = response.split("\r\n").collect();

    if lines.len() < 2 {
        return ParsedResponse::Incomplete;
    }

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
        if lines[0].len() + 2 != response.len() {
            return ParsedResponse::Incomplete;
        }

        // special case 1 token responses
        if tokens.len() == 1 {
            match &*tokens[0] {
                "PONG" | "+PONG" => {
                    return ParsedResponse::Ok;
                }
                _ => {}
            }
        }
    }
    ParsedResponse::Unknown
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "unstable")]
    extern crate test;

    use super::parse_response;
    use cfgtypes::ParsedResponse;

    #[test]
    fn test_parse_pong() {
        let r = "PONG";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "PONG\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = "ERROR\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Unknown);
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_ok_benchmark(b: &mut test::Bencher) {
        let r = "PONG\r\n";
        b.iter(|| parse_response(r));
    }
}
