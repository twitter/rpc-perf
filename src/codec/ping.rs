// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;

#[derive(Default)]
pub struct Ping {
    common: Common,
}

impl Ping {
    pub fn new() -> Ping {
        Self {
            common: Common::new(),
        }
    }

    pub fn ping(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(b"PING\r\n");
    }
}

impl Codec for Ping {
    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

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

    fn encode(&mut self, buf: &mut BytesMut, _rng: &mut ThreadRng) {
        self.ping(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Ping::new();
            let mut buf = BytesMut::with_capacity(1024);
            buf.extend_from_slice(&message);

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
