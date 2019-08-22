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

use bytes::{Buf, BytesMut, IntoBuf};

use std::io::{BufRead, BufReader};
use std::str;

pub struct PelikanRds {}

impl PelikanRds {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get(&self, buf: &mut BytesMut, key: &[u8]) {
        buf.extend_from_slice(format!("*2\r\n$3\r\nget\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn set(&self, buf: &mut BytesMut, key: &[u8], value: &[u8], ttl: Option<usize>) {
        if ttl.is_some() {
            buf.extend_from_slice(b"*5\r\n");
        } else {
            buf.extend_from_slice(b"*3\r\n");
        }
        buf.extend_from_slice(format!("$3\r\nset\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(format!("\r\n${}\r\n", value.len()).as_bytes());
        buf.extend_from_slice(value);
        buf.extend_from_slice(b"\r\n");
        if let Some(ttl_value) = ttl {
            let formated_ttl = format!("{}", ttl_value);
            buf.extend_from_slice(b"$2\r\nEX\r\n");
            buf.extend_from_slice(format!("${}\r\n", formated_ttl.len()).as_bytes());
            buf.extend_from_slice(formated_ttl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }

    pub fn sarray_create(
        &self,
        buf: &mut BytesMut,
        key: &[u8],
        esize: usize,
        watermark_low: Option<usize>,
        watermark_high: Option<usize>,
    ) {
        let esize = format!("{}", esize);
        if watermark_low.is_some() && watermark_high.is_some() {
            buf.extend_from_slice(
                format!("*5\r\n$13\r\nSArray.create\r\n${}\r\n", key.len()).as_bytes(),
            );
        } else {
            buf.extend_from_slice(
                format!("*3\r\n$13\r\nSArray.create\r\n${}\r\n", key.len()).as_bytes(),
            );
        }
        buf.extend_from_slice(key);
        buf.extend_from_slice(format!("\r\n${}\r\n{}\r\n", esize.len(), esize).as_bytes());
        if watermark_low.is_some() && watermark_high.is_some() {
            let watermark_low = format!("{}", watermark_low.unwrap());
            let watermark_high = format!("{}", watermark_high.unwrap());
            buf.extend_from_slice(
                format!("${}\r\n{}\r\n", watermark_low.len(), watermark_low).as_bytes(),
            );
            buf.extend_from_slice(
                format!("${}\r\n{}\r\n", watermark_high.len(), watermark_high).as_bytes(),
            );
        }
    }

    pub fn sarray_delete(&self, buf: &mut BytesMut, key: &[u8]) {
        buf.extend_from_slice(
            format!("*2\r\n$13\r\nSArray.delete\r\n${}\r\n", key.len()).as_bytes(),
        );
        buf.extend_from_slice(key);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_len(&self, buf: &mut BytesMut, key: &[u8]) {
        buf.extend_from_slice(format!("*2\r\n$10\r\nSArray.len\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_find(&self, buf: &mut BytesMut, key: &[u8], value: &[u8]) {
        buf.extend_from_slice(format!("*3\r\n$11\r\nSArray.find\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(format!("\r\n${}\r\n", value.len()).as_bytes());
        buf.extend_from_slice(value);
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_get(
        &self,
        buf: &mut BytesMut,
        key: &[u8],
        index: Option<u64>,
        count: Option<u64>,
    ) {
        let index = if count.is_some() && index.is_none() {
            Some("0".to_string())
        } else {
            index.map(|v| format!("{}", v))
        };
        let count = count.map(|v| format!("{}", v));
        if index.is_some() && count.is_some() {
            buf.extend_from_slice(b"*4\r\n");
        } else if index.is_some() {
            buf.extend_from_slice(b"*3\r\n");
        } else {
            buf.extend_from_slice(b"*2\r\n");
        }
        buf.extend_from_slice(format!("$10\r\nSArray.get\r\n${}\r\n", key.len()).as_bytes());
        buf.extend_from_slice(key);
        if let Some(index) = index {
            buf.extend_from_slice(format!("\r\n${}\r\n{}", index.len(), index).as_bytes());
        }
        if let Some(count) = count {
            buf.extend_from_slice(format!("\r\n${}\r\n{}", count.len(), count).as_bytes());
        }
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_insert(&self, buf: &mut BytesMut, key: &[u8], values: &Vec<&[u8]>) {
        let args = 2 + values.len();
        buf.extend_from_slice(
            format!("*{}\r\n$13\r\nSArray.insert\r\n${}\r\n", args, key.len()).as_bytes(),
        );
        buf.extend_from_slice(key);
        for value in values {
            buf.extend_from_slice(format!("\r\n${}\r\n", value.len()).as_bytes());
            buf.extend_from_slice(value);
        }
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_remove(&self, buf: &mut BytesMut, key: &[u8], values: &Vec<&[u8]>) {
        let args = 2 + values.len();
        buf.extend_from_slice(
            format!("*{}\r\n$13\r\nSArray.remove\r\n${}\r\n", args, key.len()).as_bytes(),
        );
        buf.extend_from_slice(key);
        for value in values {
            buf.extend_from_slice(format!("\r\n${}\r\n", value.len()).as_bytes());
            buf.extend_from_slice(value);
        }
        buf.extend_from_slice(b"\r\n");
    }

    pub fn sarray_truncate(&self, buf: &mut BytesMut, key: &[u8], count: u64) {
        let count = format!("{}", count);
        buf.extend_from_slice(
            format!("*3\r\n$15\r\nSArray.truncate\r\n${}\r\n", key.len()).as_bytes(),
        );
        buf.extend_from_slice(key);
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(format!("${}\r\n{}\r\n", count.len(), count).as_bytes());
    }
}

impl Decoder for PelikanRds {
    fn decode(&self, buf: &[u8]) -> Result<Response, Error> {
        let end = &buf[buf.len() - 2..buf.len()];

        // All complete responses end in CRLF
        if &end[..] != b"\r\n" {
            return Err(Error::Incomplete);
        }

        let first_char = &buf[0..1];
        match str::from_utf8(&first_char[..]) {
            Ok("+") => {
                // simple string
                if buf.len() < 5 {
                    Err(Error::Incomplete)
                } else {
                    let msg = &buf[1..buf.len() - 2];
                    match str::from_utf8(&msg[..]) {
                        Ok("OK") | Ok("PONG") | Ok("NOOP") => Ok(Response::Ok),
                        _ => Err(Error::Unknown),
                    }
                }
            }
            Ok("-") => {
                // error response
                Err(Error::Error)
            }
            Ok(":") => {
                // numeric response
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok(msg) => match msg.parse::<i64>() {
                        Ok(_) => Ok(Response::Ok),
                        Err(_) => Err(Error::Unknown),
                    },
                    Err(_) => Err(Error::Unknown),
                }
            }
            Ok("$") => {
                // bulk string
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok("-1") => Ok(Response::Miss),
                    Ok(_) => {
                        let reader = BufReader::new(buf.into_buf().reader());
                        let mut lines = reader.lines();
                        let mut line = lines.next().unwrap().unwrap();
                        let _ = line.remove(0);
                        match line.parse::<usize>() {
                            Ok(expected) => {
                                // data len = buf.len() - line.len() - 2x CRLF - 1
                                let have = buf.len() - line.len() - 5;
                                if have < expected {
                                    Err(Error::Incomplete)
                                } else if have > expected {
                                    println!("have: {} expected: {}", have, expected);
                                    Err(Error::Error)
                                } else {
                                    Ok(Response::Hit)
                                }
                            }
                            Err(_) => Err(Error::Unknown),
                        }
                    }
                    Err(_) => Err(Error::Unknown),
                }
            }
            Ok("*") => {
                // arrays
                let msg = &buf[1..buf.len() - 2];
                match str::from_utf8(&msg[..]) {
                    Ok("-1") => Ok(Response::Miss),
                    Ok(_) => {
                        // TODO: implement array parsing
                        Err(Error::Unknown)
                    }
                    Err(_) => Err(Error::Unknown),
                }
            }
            _ => Err(Error::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::*;

    fn decode_messages(messages: Vec<&'static [u8]>, response: Result<Response, Error>) {
        for message in messages {
            let decoder = PelikanRds::new();
            let mut buf = BytesMut::with_capacity(1024);
            buf.put(&message);

            let buf = buf.freeze();
            let result = decoder.decode(&buf);
            assert_eq!(result, response);
        }
    }

    #[test]
    fn decode_incomplete() {
        let messages: Vec<&[u8]> = vec![b"+OK", b"+OK\r", b"$7\r\nHELLO\r\n"];
        decode_messages(messages, Err(Error::Incomplete));
    }

    #[test]
    fn decode_ok() {
        let messages: Vec<&[u8]> = vec![b"+OK\r\n", b":12345\r\n", b"+NOOP\r\n", b"+PONG\r\n"];
        decode_messages(messages, Ok(Response::Ok));
    }

    #[test]
    fn decode_miss() {
        let messages: Vec<&[u8]> = vec![b"$-1\r\n", b"*-1\r\n"];
        decode_messages(messages, Ok(Response::Miss));
    }

    #[test]
    fn decode_error() {
        let messages: Vec<&[u8]> = vec![b"-ERROR\r\n", b"-ERROR with message\r\n"];
        decode_messages(messages, Err(Error::Error));
    }

    #[test]
    fn decode_unknown() {
        let messages: Vec<&[u8]> = vec![b"?OK\r\n", b":OK\r\n"];
        decode_messages(messages, Err(Error::Unknown));
    }

    #[test]
    fn decode_hit() {
        let messages: Vec<&[u8]> = vec![b"$0\r\n\r\n", b"$1\r\n1\r\n", b"$8\r\nDEADBEEF\r\n"];
        decode_messages(messages, Ok(Response::Hit));
    }

    #[test]
    fn encode_ttl() {
        let c = PelikanRds::new();
        let mut buf = BytesMut::with_capacity(64);
        let mut test_case = BytesMut::with_capacity(64);
        test_case.extend_from_slice(
            b"*5\r\n$3\r\nset\r\n$3\r\nabc\r\n$4\r\n1234\r\n$2\r\nEX\r\n$4\r\n9876\r\n",
        );
        c.set(&mut buf, b"abc", b"1234", Some(9876));

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_without_ttl() {
        let c = PelikanRds::new();
        let mut buf = BytesMut::with_capacity(64);
        let mut test_case = BytesMut::with_capacity(64);
        test_case.extend_from_slice(b"*3\r\n$3\r\nset\r\n$3\r\nabc\r\n$4\r\n1234\r\n");
        c.set(&mut buf, b"abc", b"1234", None);

        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_create() {
        let c = PelikanRds::new();
        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*3\r\n$13\r\nSArray.create\r\n$3\r\nabc\r\n$2\r\n64\r\n");
        c.sarray_create(&mut buf, b"abc", 64, None, None);
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(
            b"*5\r\n$13\r\nSArray.create\r\n$3\r\nabc\r\n$2\r\n64\r\n$4\r\n3000\r\n$4\r\n3200\r\n",
        );
        c.sarray_create(&mut buf, b"abc", 64, Some(3000), Some(3200));
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_delete() {
        let c = PelikanRds::new();
        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*2\r\n$13\r\nSArray.delete\r\n$3\r\nabc\r\n");
        c.sarray_delete(&mut buf, b"abc");
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_get() {
        let c = PelikanRds::new();

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*2\r\n$10\r\nSArray.get\r\n$3\r\nabc\r\n");
        c.sarray_get(&mut buf, b"abc", None, None);
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*3\r\n$10\r\nSArray.get\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        c.sarray_get(&mut buf, b"abc", Some(42), None);
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case
            .extend_from_slice(b"*4\r\n$10\r\nSArray.get\r\n$3\r\nabc\r\n$2\r\n42\r\n$1\r\n8\r\n");
        c.sarray_get(&mut buf, b"abc", Some(42), Some(8));
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case
            .extend_from_slice(b"*4\r\n$10\r\nSArray.get\r\n$3\r\nabc\r\n$1\r\n0\r\n$1\r\n8\r\n");
        c.sarray_get(&mut buf, b"abc", None, Some(8));
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_insert() {
        let c = PelikanRds::new();

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*3\r\n$13\r\nSArray.insert\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        c.sarray_insert(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(
            b"*4\r\n$13\r\nSArray.insert\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n",
        );
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        c.sarray_insert(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_remove() {
        let c = PelikanRds::new();

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*3\r\n$13\r\nSArray.remove\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        let values: Vec<&[u8]> = vec![b"42"];
        c.sarray_remove(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(
            b"*4\r\n$13\r\nSArray.remove\r\n$3\r\nabc\r\n$2\r\n42\r\n$3\r\n206\r\n",
        );
        let values: Vec<&[u8]> = vec![b"42", b"206"];
        c.sarray_remove(&mut buf, b"abc", &values);
        assert_eq!(test_case, buf);
    }

    #[test]
    fn encode_sarray_truncate() {
        let c = PelikanRds::new();

        let mut buf = BytesMut::with_capacity(128);
        let mut test_case = BytesMut::with_capacity(128);
        test_case.extend_from_slice(b"*3\r\n$15\r\nSArray.truncate\r\n$3\r\nabc\r\n$2\r\n42\r\n");
        c.sarray_truncate(&mut buf, b"abc", 42);
        assert_eq!(test_case, buf);
    }
}
