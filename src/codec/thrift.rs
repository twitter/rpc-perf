// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#![allow(dead_code)]

use crate::codec::ParseError;

pub const STOP: u8 = 0;
pub const VOID: u8 = 1;
pub const BOOL: u8 = 2;
pub const BYTE: u8 = 3;
pub const DOUBLE: u8 = 4;
pub const I16: u8 = 6;
pub const I32: u8 = 8;
pub const I64: u8 = 10;
pub const STRING: u8 = 11;
pub const STRUCT: u8 = 12;
pub const MAP: u8 = 13;
pub const SET: u8 = 14;
pub const LIST: u8 = 15;

#[derive(Clone)]
pub struct ThriftBuffer {
    buffer: Vec<u8>,
}

impl Default for ThriftBuffer {
    fn default() -> Self {
        let mut buffer = Vec::<u8>::new();
        buffer.resize(4, 0);

        Self { buffer }
    }
}

impl ThriftBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
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
        for (p, i) in (bytes as i32).to_be_bytes().iter().enumerate() {
            self.buffer[p] = *i;
        }
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
        self.write_bytes(&[STOP])
    }

    // write an i16 to the buffer
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> &Self {
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    // write an i32 to the buffer
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> &Self {
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    // write an i64 to the buffer
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> &Self {
        self.buffer.extend_from_slice(&value.to_be_bytes());
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

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

fn decode(buf: &[u8]) -> Result<(), ParseError> {
    let bytes = buf.len() as u32;
    if bytes > 4 {
        let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);

        match length.checked_add(4_u32) {
            Some(b) => {
                if b == bytes {
                    Ok(())
                } else {
                    Err(ParseError::Incomplete)
                }
            }
            None => Err(ParseError::Unknown),
        }
    } else {
        Err(ParseError::Incomplete)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ping() {
        let mut buffer = ThriftBuffer::new();

        // new buffer has 4 bytes to hold framing later
        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer.as_bytes(), &[0, 0, 0, 0]);

        buffer.protocol_header();
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer.as_bytes(), &[0, 0, 0, 0, 128, 1, 0, 1]);

        buffer.method_name("ping");
        assert_eq!(buffer.len(), 16);
        assert_eq!(
            buffer.as_bytes(),
            &[0, 0, 0, 0, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103]
        );

        buffer.sequence_id(0);
        assert_eq!(buffer.len(), 20);
        assert_eq!(
            buffer.as_bytes(),
            &[0, 0, 0, 0, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0]
        );

        buffer.stop();
        assert_eq!(buffer.len(), 21);
        assert_eq!(
            buffer.as_bytes(),
            &[0, 0, 0, 0, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0]
        );

        buffer.frame();
        assert_eq!(buffer.len(), 21);
        assert_eq!(
            buffer.as_bytes(),
            &[0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0]
        );

        assert_eq!(decode(buffer.as_bytes()), Ok(()));
    }
}
