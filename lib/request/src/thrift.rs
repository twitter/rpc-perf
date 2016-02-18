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
/// let mut msg = vec![1, 3, 3, 7];
/// frame(&mut msg);
/// let expected = vec![0, 0, 0, 4, 1, 3, 3, 7];
/// assert_eq!(msg, expected);
pub fn frame(msg: &mut Vec<u8>) {
    // get length of msg
    let bytes = msg.len();

    // extend the msg to store the i32 size
    msg.resize(bytes + 4, 0);

    // shift the message to the right by 4
    for i in (0..bytes).rev() {
        msg[(i + 4)] = msg[i];
    }

    // write size into frame
    BigEndian::write_i32(&mut msg[..4], bytes as i32);
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
/// let method = "ping".to_string();
/// method_name(&mut buffer, method.clone());
/// let expected = vec![0, 0, 0, 4, 112, 105, 110, 103];
/// assert_eq!(buffer, expected);
pub fn method_name(buffer: &mut Vec<u8>, method: String) {
    let mut method = method.into_bytes();
    frame(&mut method);
    buffer.append(&mut method);
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
    protocol_header(&mut buffer);
    method_name(&mut buffer, "ping".to_string());
    sequence_id(&mut buffer, 0);
    stop(&mut buffer);
    frame(&mut buffer);
    buffer
}
