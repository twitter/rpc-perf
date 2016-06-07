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

pub use cfgtypes::ParsedResponse;


pub fn parse_response(response: &str) -> ParsedResponse {

    let mut lines: Vec<&str> = response.split("\r\n").collect();

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

#[cfg(test)]
mod tests {
    use cfgtypes::ParsedResponse;
    use super::parse_response;

    #[test]
    fn test_parse_incomplete() {
        let r = "+OK";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "+OK\r";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_invalid() {
        let r = "?OK\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Invalid);

        let r = ":OK\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Invalid);
    }

    #[test]
    fn test_parse_ok() {
        let r = "+OK\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = "$0\r\n\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Hit);

        let r = "$1\r\n1\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Hit);

        let r = ":12345\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = ":-12345\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = "-ERROR\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Error("ERROR".to_string()));

        let r = "-ERROR with message\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Error("ERROR with message".to_string()));
    }

    #[test]
    fn test_parse_miss() {
        let r = "$-1\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Miss);

        let r = "*-1\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Miss);
    }
}
