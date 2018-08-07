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

use super::consts;
use byteorder::{BigEndian, ByteOrder, WriteBytesExt};

#[derive(Clone)]
pub struct Buffer {
    buffer: Vec<u8>,
}

impl Default for Buffer {
    fn default() -> Buffer {
        let mut buffer = Vec::<u8>::new();
        buffer.resize(4, 0);

        Buffer { buffer: buffer }
    }
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer::default()
    }

    /// returns the Vec<u8> from the `Buffer`
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// add protocol version to buffer
    pub fn protocol_header(&mut self) -> &Self {
        self.buffer.extend_from_slice(&[128, 1, 0, 1]);
        self
    }

    /// write the framed length to the buffer
    #[inline]
    pub fn frame(&mut self) -> &Self {
        let bytes = self.buffer.len() - 4;
        BigEndian::write_i32(&mut self.buffer[..4], bytes as i32);
        self
    }

    /// add method name to buffer
    #[inline]
    pub fn method_name(&mut self, method: &str) -> &Self {
        self.write_str(method)
    }

    /// add sequence id to buffer
    #[inline]
    pub fn sequence_id(&mut self, id: i32) -> &Self {
        self.write_i32(id as i32)
    }

    /// add stop sequence to buffer
    pub fn stop(&mut self) -> &Self {
        self.write_bytes(&[consts::STOP])
    }

    // write an i16 to the buffer
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> &Self {
        self.buffer.write_i16::<BigEndian>(value).unwrap();
        self
    }

    // write an i32 to the buffer
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> &Self {
        self.buffer.write_i32::<BigEndian>(value).unwrap();
        self
    }

    // write an i64 to the buffer
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> &Self {
        self.buffer.write_i64::<BigEndian>(value).unwrap();
        self
    }

    #[inline]
    pub fn write_f64(&mut self, value: f64) -> &Self {
        self.buffer.write_f64::<BigEndian>(value).unwrap();
        self
    }

    // write a literal byte sequence to the buffer
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) -> &Self {
        self.buffer.extend_from_slice(bytes);
        self
    }

    // write bool to the buffer
    #[inline]
    pub fn write_bool(&mut self, b: bool) -> &Self {
        self.buffer.extend_from_slice(&[(b as u8)]);
        self
    }

    #[inline]
    pub fn write_str(&mut self, string: &str) -> &Self {
        let string = string.as_bytes();
        self.write_i32(string.len() as i32);
        self.buffer.extend_from_slice(string);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buff<F>(expected: &[u8], f: F)
    where
        F: FnOnce(&mut Buffer) -> (),
    {
        let mut b = Buffer::new();
        f(&mut b);
        assert_eq!(b.into_vec().as_slice(), expected);
    }

    #[test]
    fn into_vec() {
        test_buff(&[0, 0, 0, 0], |_| {});
    }

    #[test]
    fn test_protocol_header() {
        test_buff(&[0, 0, 0, 0, 128, 1, 0, 1], |b| { b.protocol_header(); });
    }

    #[test]
    fn test_sequence_id() {
        test_buff(&[0, 0, 0, 4, 0, 0, 0, 0], |b| {
            b.sequence_id(0_i32);
            b.frame();
        });
    }

    #[test]
    fn test_method_name() {
        test_buff(&[0, 0, 0, 0, 0, 0, 0, 4, 112, 105, 110, 103], |b| {
            b.method_name("ping");
        });
    }

    #[test]
    fn test_stop() {
        test_buff(&vec![0, 0, 0, 0, 0], |b| { b.stop(); });
    }
}
