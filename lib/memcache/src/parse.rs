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
                "OK" | "STORED" | "DELETED" => {
                    return ParsedResponse::Ok;
                }
                "END" | "EXISTS" | "NOT_FOUND" | "NOT_STORED" => {
                    return ParsedResponse::Miss;
                }
                "VALUE" => {
                    return ParsedResponse::Incomplete;
                }
                "ERROR" => {
                    return ParsedResponse::Error(response.to_owned());
                }
                _ => {}
            }
            // incr/decr give a numeric single token response
            if let Ok(_) = tokens[0].parse::<u64>() {
                return ParsedResponse::Ok;
            }
        } else {
            match &*tokens[0] {
                "VALUE" => {
                    return ParsedResponse::Incomplete;
                }
                "VERSION" => {
                    let v: String = tokens[1..tokens.len()].join(" ");
                    return ParsedResponse::Version(v);
                }
                "CLIENT_ERROR" | "SERVER_ERROR" => {
                    return ParsedResponse::Error(response.to_owned());
                }
                _ => {
                    return ParsedResponse::Unknown;
                }
            }
        }
    } else {
        match &*tokens[0] {
            "VALUE" => {
                if tokens.len() < 4 {
                    return ParsedResponse::Incomplete;
                }
                let bytes = tokens[3];
                if tokens.len() == 5 {
                    match tokens[4].parse::<u64>() {
                        Ok(_) => {}
                        Err(_) => {
                            return ParsedResponse::Invalid;
                        }
                    }
                }
                match tokens[2].parse::<u32>() {
                    Ok(_) => {}
                    Err(_) => {
                        return ParsedResponse::Invalid;
                    }
                }
                if lines[lines.len() - 1] != "END" {
                    // END is always final line of complete response
                    return ParsedResponse::Incomplete;
                }
                let data = lines[1..lines.len() - 1].join("\r\n"); //reinsert any CRLF in data
                match bytes.parse() {
                    Ok(b) => {
                        if data.len() == b {
                            // we have correct length data section
                            return ParsedResponse::Hit;
                        }
                        if data.len() > b {
                            // more data than in bytes field
                            return ParsedResponse::Invalid;
                        }
                    }
                    Err(_) => {
                        // bytes field failed to parse to usize
                        return ParsedResponse::Invalid;
                    }
                }
                return ParsedResponse::Incomplete;
            }
            _ => {
                return ParsedResponse::Unknown;
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
    fn test_parse_incomplete() {
        let r = "0";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "STOR";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "STORED";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "STORED\r";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VERSION ";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VERSION 1.2.3";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VERSION 1.2.3\r";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "CLIENT_ERROR";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "SERVER_ERROR error msg";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VALUE key 0 1 0\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VALUE key 0 10\r\n0123456789\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VALUE key 0 10\r\n0123456789\r\nEND\r";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);

        let r = "VALUE key 0 10\r\nEND\r\nEND\r\n\r\nEND";
        assert_eq!(parse_response(r), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_invalid() {
        let r = "VALUE key 0 10\r\n0123456789ABCDEF\r\nEND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Invalid);

        let r = "VALUE key 0 NaN\r\n0123456789ABCDEF\r\nEND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Invalid);

        let r = "VALUE key NaN 10\r\n0123456789\r\nEND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Invalid);
    }

    #[test]
    fn test_parse_ok() {
        let r = "OK\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = "STORED\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = "DELETED\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);

        let r = "VALUE key 0 10\r\n0123456789\r\nEND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Hit);

        let r = "VALUE key 0 10\r\n0123456789\r\nEND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Hit);

        let r = "12345\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = "ERROR\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Error("ERROR\r\n".to_owned()));

        let r = "CLIENT_ERROR some message\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Error("CLIENT_ERROR some message\r\n".to_owned()));

        let r = "SERVER_ERROR some message\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Error("SERVER_ERROR some message\r\n".to_owned()));
    }

    #[test]
    fn test_parse_miss() {
        let r = "EXISTS\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Miss);

        let r = "NOT_FOUND\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Miss);

        let r = "NOT_STORED\r\n";
        assert_eq!(parse_response(r), ParsedResponse::Miss);
    }

    #[test]
    fn test_parse_version() {
        let r = "VERSION 1.2.3\r\n";
        assert_eq!(parse_response(r),
                   ParsedResponse::Version("1.2.3".to_owned()));
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_hit_benchmark(b: &mut test::Bencher) {
        let r = "VALUE key 0 10\r\n0123456789\r\nEND\r\n";
        b.iter(|| parse_response(r));
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn parse_miss_benchmark(b: &mut test::Bencher) {
        let r = "NOT_FOUND\r\n";
        b.iter(|| parse_response(r));
    }
}
