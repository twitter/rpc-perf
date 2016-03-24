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

extern crate byteorder;

use byteorder::{ByteOrder, BigEndian};

/// this is work in progress to speak framed binary thrift

pub struct Buffer {
    buffer: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let mut buffer = Vec::<u8>::new();
        buffer.resize(4, 0);

        Buffer { buffer: buffer }
    }

    pub fn buffer(&mut self) -> Vec<u8> {
        self.buffer.clone()
    }

    /// add protocol version to buffer
    ///
    /// # Example
    /// ```
    /// # use rpcperf_request::thrift::*;
    ///
    // let mut buffer = Vec::<u8>::new();
    // protocol_header(&mut buffer);
    // let expected = vec![128, 1, 0, 1];
    // assert_eq!(buffer, expected);
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
    /// assert_eq!(b.buffer(), expected);
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
    /// b.method_name("ping".to_string());
    /// let expected = vec![0, 0, 0, 0, 0, 0, 0, 4, 112, 105, 110, 103];
    /// assert_eq!(b.buffer(), expected);
    pub fn method_name(&mut self, method: String) -> &Self {
        self.write_string(method)
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
    /// assert_eq!(b.buffer(), expected);
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
    /// assert_eq!(b.buffer(), expected);
    pub fn stop(&mut self) -> &Self {
        self.write_bytes(&[0])
    }


    fn write_i32(&mut self, value: i32) -> &Self {
        let bytes = self.buffer.len();
        self.buffer.resize(bytes + 4, 0);
        BigEndian::write_i32(&mut self.buffer[bytes..], value);
        self
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> &Self {
        self.buffer.extend_from_slice(bytes);
        self
    }

    fn write_string(&mut self, string: String) -> &Self {
        let mut string = string.into_bytes();
        self.write_i32(string.len() as i32);
        self.buffer.append(&mut string);
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
    let mut buffer = Buffer::new();
    buffer.protocol_header();
    buffer.method_name("ping".to_owned());
    buffer.sequence_id(0);
    buffer.stop();
    buffer.frame();
    buffer.buffer()
}
