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

use byteorder::{ByteOrder, BigEndian, WriteBytesExt};
use workload::{Value, Parameter};

const STOP: u8 = 0;
const VOID: u8 = 1;
const BOOL: u8 = 2;
const BYTE: u8 = 3;
const DOUBLE: u8 = 4;
const I16: u8 = 6;
const I32: u8 = 8;
const I64: u8 = 10;
const STRING: u8 = 11;
const STRUCT: u8 = 12;
const MAP: u8 = 13;
const SET: u8 = 14;
const LIST: u8 = 15;

#[derive(Clone)]
pub struct Buffer {
    buffer: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let mut buffer = Vec::<u8>::new();
        buffer.resize(4, 0);

        Buffer { buffer: buffer }
    }

    /// returns the Vec<u8> from the `Buffer`
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::*;
    ///
    /// let mut b = Buffer::new();
    /// let expected = vec![0, 0, 0, 0];
    /// assert_eq!(b.into_vec(), expected);
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// add protocol version to buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::*;
    ///
    /// let mut b = Buffer::new();
    /// b.protocol_header();
    /// let expected = vec![0, 0, 0, 0, 128, 1, 0, 1];
    /// assert_eq!(b.into_vec(), expected);
    pub fn protocol_header(&mut self) -> &Self {
        self.buffer.extend_from_slice(&[128, 1, 0, 1]);
        self
    }

    /// write the framed length to the buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::*;
    ///
    /// let mut b = Buffer::new();
    /// b.sequence_id(0_i32);
    /// b.frame();
    /// let expected = vec![0, 0, 0, 4, 0, 0, 0, 0];
    /// assert_eq!(b.into_vec(), expected);
    #[inline]
    pub fn frame(&mut self) -> &Self {
        let bytes = self.buffer.len() - 4;
        BigEndian::write_i32(&mut self.buffer[..4], bytes as i32);
        self
    }

    /// add method name to buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::Buffer;
    ///
    /// let mut b = Buffer::new();
    /// b.method_name("ping");
    /// let expected = vec![0, 0, 0, 0, 0, 0, 0, 4, 112, 105, 110, 103];
    /// assert_eq!(b.into_vec(), expected);
    #[inline]
    pub fn method_name(&mut self, method: &str) -> &Self {
        self.write_str(method)
    }

    /// add sequence id to buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::Buffer;
    ///
    /// let mut b = Buffer::new();
    /// b.sequence_id(0_i32);
    /// let expected = vec![0, 0, 0, 0, 0, 0, 0, 0];
    /// assert_eq!(b.into_vec(), expected);
    #[inline]
    pub fn sequence_id(&mut self, id: i32) -> &Self {
        self.write_i32(id as i32)
    }

    /// add stop sequence to buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::Buffer;
    ///
    /// let mut b = Buffer::new();
    /// b.stop();
    /// let expected = vec![0, 0, 0, 0, 0];
    /// assert_eq!(b.into_vec(), expected);
    pub fn stop(&mut self) -> &Self {
        self.write_bytes(&[STOP])
    }

    // write an i16 to the buffer
    #[inline]
    fn write_i16(&mut self, value: i16) -> &Self {
        self.buffer.write_i16::<BigEndian>(value).unwrap();
        self
    }

    // write an i32 to the buffer
    #[inline]
    fn write_i32(&mut self, value: i32) -> &Self {
        self.buffer.write_i32::<BigEndian>(value).unwrap();
        self
    }

    // write an i64 to the buffer
    #[inline]
    fn write_i64(&mut self, value: i64) -> &Self {
        self.buffer.write_i64::<BigEndian>(value).unwrap();
        self
    }

    #[inline]
    fn write_f64(&mut self, value: f64) -> &Self {
        self.buffer.write_f64::<BigEndian>(value).unwrap();
        self
    }

    // write a literal byte sequence to the buffer
    #[inline]
    fn write_bytes(&mut self, bytes: &[u8]) -> &Self {
        self.buffer.extend_from_slice(bytes);
        self
    }

    // write bool to the buffer
    #[inline]
    fn write_bool(&mut self, b: bool) -> &Self {
        self.buffer.extend_from_slice(&[(b as u8)]);
        self
    }

    #[inline]
    fn write_str(&mut self, string: &str) -> &Self {
        let string = string.as_bytes();
        self.write_i32(string.len() as i32);
        self.buffer.extend_from_slice(string);
        self
    }
}

/// create a ping request
///
/// # Example
/// ```
/// # use rpcperf_request::thrift::*;
///
/// assert_eq!(ping(), [0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0]);
pub fn ping() -> Vec<u8> {
    generic("ping", 0, &Vec::new())
}

pub fn generic(method: &str, sequence_id: i32, payload: &Vec<Parameter>) -> Vec<u8> {
    let mut buffer = Buffer::new();
    buffer.protocol_header();
    buffer.method_name(method);
    buffer.sequence_id(sequence_id);
    for p in payload {
        match p.value {
            Value::Stop => {
                buffer.stop();
            }
            Value::Void => {
                buffer.write_bytes(&[VOID]);
            }
            Value::Bool(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[BOOL]);
                    buffer.write_i16(id);
                }
                buffer.write_bool(val);
            }
            Value::Byte(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[BYTE]);
                    buffer.write_i16(id);
                }
                buffer.write_bytes(&[val]);
            }
            Value::Double(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[DOUBLE]);
                    buffer.write_i16(id);
                }
                buffer.write_f64(val);
            }
            Value::Int16(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[I16]);
                    buffer.write_i16(id);
                }
                buffer.write_i16(val);
            }
            Value::Int32(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[I32]);
                    buffer.write_i16(id);
                }
                buffer.write_i32(val);
            }
            Value::Int64(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[I64]);
                    buffer.write_i16(id);
                }
                buffer.write_i64(val);
            }
            Value::String(ref val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[STRING]);
                    buffer.write_i16(id);
                }
                buffer.write_str(&val);
            }
            Value::Struct => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[STRUCT]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type struct must have an id");
                }
            }
            Value::Map => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[MAP]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type map must have an id");
                }
            }
            Value::Set => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[SET]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type set must have an id");
                }
            }
            Value::List(ref ttype, len) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[LIST]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type list must have an id");
                }

                // TODO: this could be better
                let byte = match ttype.as_str() {
                    "string" => STRING,
                    "struct" => STRUCT,
                    _ => {
                        panic!("unsupported ttype for list");
                    }
                };
                buffer.write_bytes(&[byte]);
                buffer.write_i32(len);
            }
            ref other => panic!("unknown thrift parameter type: {:?}", other),
        }
    }
    buffer.stop();
    buffer.frame();
    buffer.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use workload::{Value, Parameter};

    fn mk_param(id: i16, value: Value) -> Parameter {
        let mut p = Parameter::default();
        p.id = Some(id);
        p.value = value;
        p
    }

    #[test]
    fn thrift_ping() {
        assert_eq!(generic("ping", 0, &Vec::new()),
                   vec![0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0]);
    }

    // thrift calculator `add` example
    #[test]
    fn thrift_add() {
        let mut payload = Vec::new();
        payload.push(mk_param(1, Value::Int32(1)));
        payload.push(mk_param(2, Value::Int32(1)));

        assert_eq!(generic("add", 0, &payload),
                   vec![0, 0, 0, 30, 128, 1, 0, 1, 0, 0, 0, 3, 97, 100, 100, 0, 0, 0, 0, 8, 0, 1,
                        0, 0, 0, 1, 8, 0, 2, 0, 0, 0, 1, 0]);
    }

    // thrift calculator subtraction example
    #[test]
    fn thrift_subtract() {
        let mut payload = Vec::new();
        payload.push(mk_param(1, Value::Int32(1)));
        payload.push(mk_param(2, Value::Struct));
        payload.push(mk_param(1, Value::Int32(15)));
        payload.push(mk_param(2, Value::Int32(10)));
        payload.push(mk_param(3, Value::Int32(2)));
        payload.push(mk_param(0, Value::Stop));

        assert_eq!(generic("calculate", 0, &payload),
                   vec![0, 0, 0, 54, 128, 1, 0, 1, 0, 0, 0, 9, 99, 97, 108, 99, 117, 108, 97,
                        116, 101, 0, 0, 0, 0, 8, 0, 1, 0, 0, 0, 1, 12, 0, 2, 8, 0, 1, 0, 0, 0,
                        15, 8, 0, 2, 0, 0, 0, 10, 8, 0, 3, 0, 0, 0, 2, 0, 0]);
    }
}
