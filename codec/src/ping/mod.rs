//  Copyright 2019 Twitter, Inc
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

use super::*;

use bytes::BytesMut;

#[derive(Default)]
pub struct Ping {}

impl Ping {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ping(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(b"PING\r\n");
    }
}

impl Decoder for Ping {
    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        if buf.len() < 6 || &buf[buf.len() - 2..buf.len()] != b"\r\n" {
            // Shortest response is "PONG\r\n" at 4bytes
            // All complete responses end in CRLF
            Err(Error::Incomplete)
        } else if (buf.len() == 6 && &buf[..] == b"PONG\r\n")
            || (buf.len() == 7 && &buf[..] == b"+PONG\r\n")
        {
            Ok(Response::Ok)
        } else {
            Err(Error::Unknown)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BufMut;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Ping::new();
            let mut buf = BytesMut::with_capacity(1024);
            buf.put(&message);

            let buf = buf.freeze();
            let result = decoder.decode(&buf);
            assert_eq!(result, response);
        }
    }

    #[test]
    fn decode_incomplete() {
        let messages: Vec<&[u8]> = vec![b"", b"PONG", b"+PONG", b"P"];
        decode_messages(messages, Err(Error::Incomplete));
    }

    #[test]
    fn decode_ok() {
        let messages: Vec<&[u8]> = vec![b"PONG\r\n", b"+PONG\r\n"];
        decode_messages(messages, Ok(Response::Ok));
    }

    #[test]
    fn decode_unknown() {
        let messages: Vec<&[u8]> = vec![
            b"HELLO WORLD\r\n",
            b"+PONG\r\nDEADBEEF\r\n",
            b"+PONG PONG\r\n",
        ];
        decode_messages(messages, Err(Error::Unknown));
    }

    #[test]
    fn encode_ping() {
        let mut buf = BytesMut::new();
        let encoder = Ping::new();
        encoder.ping(&mut buf);
        assert_eq!(&buf[..], b"PING\r\n");
    }
}
