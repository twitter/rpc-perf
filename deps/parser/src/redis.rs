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

pub use super::*;

pub struct Response {
    pub response: String,
}

impl Parse for Response {
    fn parse(&self) -> ParsedResponse {

        let mut lines: Vec<&str> = self.response.split("\r\n").collect();

        // expect an empty line from the split
        if lines[lines.len() - 1].len() == 0 {
            let _ = lines.pop();
        } else {
            return ParsedResponse::Incomplete;
        }

        let mut bytes: Vec<u8> = lines[0].bytes().collect();

        let first_byte = bytes.remove(0);

        let msg = String::from_utf8(bytes).unwrap();

        match first_byte {
            43 => {
                // + simple string
                match &*msg {
                    "OK" => {
                        return ParsedResponse::Ok;
                    }
                    "PONG" => {
                        return ParsedResponse::Ok;
                    }
                    _ => {}
                }
            }
            45 => {
                // - errors
                return ParsedResponse::Error(msg);
            }
            58 => {
                // : integers
                match msg.parse::<i64>() {
                    Ok(_) => {
                        return ParsedResponse::Ok;
                    }
                    Err(_) => {
                        return ParsedResponse::Invalid;
                    }
                }
            }
            36 => {
                // $ bulk strings
                if &msg == "-1" {
                    return ParsedResponse::Miss;
                }
                match msg.parse() {
                    Ok(bytes) => {
                        let data = lines[1..lines.len()].join("\r\n");
                        if data.len() == bytes {
                            return ParsedResponse::Hit;
                        }
                        if data.len() < bytes {
                            return ParsedResponse::Incomplete;
                        }
                        return ParsedResponse::Invalid;
                    }
                    Err(_) => {
                        return ParsedResponse::Invalid;
                    }
                }
            }
            42 => {
                // * arrays
                if &msg == "*-1\r\n" {
                    return ParsedResponse::Miss;
                }
                return ParsedResponse::Unknown;
            }
            _ => {
                return ParsedResponse::Invalid;
            }
        }
        ParsedResponse::Invalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_incomplete() {
        let r = Response { response: "+OK".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);

        let r = Response { response: "+OK\r".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Incomplete);
    }

    #[test]
    fn test_parse_invalid() {
        let r = Response { response: "?OK\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Invalid);

        let r = Response { response: ":OK\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Invalid);
    }

    #[test]
    fn test_parse_ok() {
        let r = Response { response: "+OK\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Ok);

        let r = Response { response: "$0\r\n\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Hit);

        let r = Response { response: "$1\r\n1\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Hit);

        let r = Response { response: ":12345\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Ok);

        let r = Response { response: ":-12345\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let r = Response { response: "-ERROR\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Error("ERROR".to_string()));

        let r = Response { response: "-ERROR with message\r\n".to_string() };
        assert_eq!(r.parse(),
                   ParsedResponse::Error("ERROR with message".to_string()));
    }

    #[test]
    fn test_parse_miss() {
        let r = Response { response: "$-1\r\n".to_string() };
        assert_eq!(r.parse(), ParsedResponse::Miss);
    }
}
