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

/// takes a message and frames it
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// let mut msg = Vec::new();
/// frame(&mut msg, |mut msg| {
///     msg.extend_from_slice(&[1, 3, 3, 7]);
/// });
/// let expected = vec![0, 0, 0, 4, 1, 3, 3, 7];
/// assert_eq!(msg, expected);
pub fn frame<F,R>(buffer: &mut Vec<u8>, f: F) -> R
    where F: Fn(&mut Vec<u8>) -> R {
    
    let orig_size = buffer.len();
    let frame_start = orig_size + 4;

    buffer.resize(frame_start, 0);
    let result = f(buffer);

    let new_size = buffer.len();

    // the closure shouldn't shrink the buffer
    assert!(new_size >= frame_start);

    let len = new_size - frame_start;
    BigEndian::write_i32(&mut buffer[orig_size..frame_start], len as i32);

    result
}

/// add Binary Protocol version to buffer
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// let mut buffer = Vec::<u8>::new();
/// protocol_header(&mut buffer);
/// let expected = vec![128, 1, 0, 1];
/// assert_eq!(buffer, expected);
pub fn protocol_header(buffer: &mut Vec<u8>) {
    // get length of msg
    buffer.extend_from_slice(&[128, 1, 0, 1]);
}

/// add method name to buffer
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// let mut buffer = Vec::<u8>::new();
/// method_name(&mut buffer, "ping");
/// let expected = vec![0, 0, 0, 4, 112, 105, 110, 103];
/// assert_eq!(buffer, expected);
pub fn method_name(buffer: &mut Vec<u8>, method: &str) {
    frame(buffer, |mut buff| {
        buff.extend_from_slice(method.as_bytes());
    });
}

/// add sequence id to buffer
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// let mut buffer = Vec::<u8>::new();
/// sequence_id(&mut buffer, 0_i32);
/// let expected = vec![0, 0, 0, 0];
/// assert_eq!(buffer, expected);
pub fn sequence_id(buffer: &mut Vec<u8>, id: i32) {
    let bytes = buffer.len();
    buffer.resize(bytes + 4, 0);
    BigEndian::write_i32(&mut buffer[bytes..], id);
}

/// add stop mark to buffer
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// let mut buffer = Vec::<u8>::new();
/// stop(&mut buffer);
/// let expected = vec![0];
/// assert_eq!(buffer, expected);
pub fn stop(buffer: &mut Vec<u8>) {
    buffer.push(0);
}

/// create a ping request
///
/// # Example
/// ```
/// # use request::thrift::*;
///
/// assert_eq!(ping(), [0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0]);
pub fn ping() -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();

    frame(&mut buffer, |mut buffer| {
        protocol_header(&mut buffer);
        method_name(&mut buffer, "ping");
        sequence_id(&mut buffer, 0);
        stop(&mut buffer);
    });

    buffer
}
