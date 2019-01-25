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

use bytes::{BufMut, BytesMut};

use std::mem::transmute;

#[derive(Default)]
pub struct Echo {}

impl Echo {
    pub fn new() -> Self {
        Self {}
    }

    pub fn echo(&self, buf: &mut BytesMut, value: &[u8]) {
        let crc = crc::crc32::checksum_ieee(value);
        buf.extend_from_slice(value);
        buf.put_u32_be(crc); // TODO: this could panic
        buf.extend_from_slice(b"\r\n");
    }
}

impl Decoder for Echo {
    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        // shortest response is 7 bytes (1 byte + 4 byte crc + CR + LF)
        if buf.len() < 7 {
            return Err(Error::Incomplete);
        }

        let end = &buf[buf.len() - 2..buf.len()];

        // All complete responses end in CRLF
        if &end[..] != b"\r\n" {
            return Err(Error::Incomplete);
        }

        let crc = &buf[buf.len() - 6..buf.len() - 2];

        let message = &buf[0..buf.len() - 6];

        let crc_calc = crc::crc32::checksum_ieee(&message[..]);
        let crc_bytes: [u8; 4] = unsafe { transmute(crc_calc.to_be()) };
        if crc_bytes != crc[..] {
            Err(Error::ChecksumMismatch(
                crc[..].to_owned(),
                crc_bytes.to_vec(),
            ))
        } else {
            Ok(Response::Ok)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use bytes::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = Echo::new();
            let mut buf = BytesMut::with_capacity(1024);
            buf.put(&message);

            let buf = buf.freeze();
            let result = decoder.decode(&buf);
            assert_eq!(result, response);
        }
    }

    #[test]
    fn decode_incomplete() {
        let messages: Vec<&[u8]> = vec![b""];
        decode_messages(messages, Err(Error::Incomplete));
    }

    #[test]
    fn decode_ok() {
        let messages: Vec<&[u8]> = vec![&[0, 1, 2, 8, 84, 137, 127, 13, 10]];
        decode_messages(messages, Ok(Response::Ok));
    }

    #[test]
    fn decode_checksum_mismatch() {
        let messages: Vec<&[u8]> = vec![b"3421780262\r\n"];
        decode_messages(
            messages,
            Err(Error::ChecksumMismatch(
                vec![48, 50, 54, 50],
                vec![160, 3, 109, 193],
            )),
        );
    }

    #[test]
    fn encode_echo() {
        let mut buf = BytesMut::new();
        let encoder = Echo::new();
        encoder.echo(&mut buf, &[0, 1, 2]);
        assert_eq!(&buf[..], &[0, 1, 2, 8, 84, 137, 127, 13, 10]);
    }
}
