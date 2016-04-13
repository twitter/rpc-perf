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
        if lines[lines.len() - 1].is_empty() {
            let _ = lines.pop();
        } else {
            return ParsedResponse::Incomplete;
        }

        let (first_char, msg) = lines[0].split_at(1);

        match first_char {
            "+" => {
                // simple string
                // + simple string
                match msg {
                    "OK" | "PONG" => ParsedResponse::Ok,
                    _ => ParsedResponse::Invalid,
                }
            }
            "-" => {
                // errors
                ParsedResponse::Error(msg.to_owned())
            }
            ":" => {
                // integers
                match msg.parse::<i64>() {
                    Ok(_) => ParsedResponse::Ok,
                    Err(_) => ParsedResponse::Invalid,
                }
            }
            "$" if msg == "-1" => ParsedResponse::Miss,
            "$" => {
                match msg.parse() {
                    Ok(bytes) => {
                        let data = lines[1..lines.len()].join("\r\n");
                        if data.len() == bytes {
                            ParsedResponse::Hit
                        } else if data.len() < bytes {
                            ParsedResponse::Incomplete
                        } else {
                            ParsedResponse::Invalid
                        }
                    }
                    Err(_) => ParsedResponse::Invalid,
                }
            }
            // arrays
            "*" if msg == "-1" => ParsedResponse::Miss,
            "*" => ParsedResponse::Unknown,

            // Unknown type
            _ => ParsedResponse::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Parse, ParsedResponse, Response};

    #[cfg(feature = "unstable")]
    extern crate test;

    #[test]
    fn test_parse_incomplete() {
        let r = Response { response: "+OK" };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);

        let r = Response { response: "+OK\r" };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_invalid() {
        let r = Response { response: "?OK\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Invalid);

        let r = Response { response: ":OK\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Invalid);
    }

    #[test]
    fn test_parse_ok() {
        let r = Response { response: "+OK\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Ok);

        let r = Response { response: "$0\r\n\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Hit);

        let r = Response { response: "$1\r\n1\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Hit);

        let r = Response { response: ":12345\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Ok);

        let r = Response { response: ":-12345\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = Response { response: "-ERROR\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Error("ERROR".to_string()));

        let r = Response { response: "-ERROR with message\r\n" };
        assert_eq!(r.parse(),
                   ParsedResponse::Error("ERROR with message".to_string()));
    }

    #[test]
    fn test_parse_miss() {
        let r = Response { response: "$-1\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Miss);

        let r = Response { response: "*-1\r\n" };
        assert_eq!(r.parse(), ParsedResponse::Miss);
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_hit_benchmark(b: &mut test::Bencher) {
        let r = Response { response: "$1\r\n1\r\n" };
        b.iter(|| r.parse());
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_miss_benchmark(b: &mut test::Bencher) {
        let r = Response { response: "$-1\r\n" };
        b.iter(|| r.parse());
    }
}
