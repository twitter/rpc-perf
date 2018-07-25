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

use super::buffer::Buffer;
use super::consts;
use super::{Parameter, Tvalue};

/// create a ping request
pub fn ping() -> Vec<u8> {
    generic("ping", 0, &mut Vec::new())
}

pub fn generic(method: &str, sequence_id: i32, payload: &mut Vec<Parameter>) -> Vec<u8> {
    let mut buffer = Buffer::new();
    buffer.protocol_header();
    buffer.method_name(method);
    buffer.sequence_id(sequence_id);
    for p in payload {
        p.regen();
        match p.value {
            Tvalue::Stop => {
                buffer.stop();
            }
            Tvalue::Void => {
                buffer.write_bytes(&[consts::VOID]);
            }
            Tvalue::Bool(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::BOOL]);
                    buffer.write_i16(id);
                }
                buffer.write_bool(val);
            }
            Tvalue::Byte(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::BYTE]);
                    buffer.write_i16(id);
                }
                buffer.write_bytes(&[val]);
            }
            Tvalue::Double(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::DOUBLE]);
                    buffer.write_i16(id);
                }
                buffer.write_f64(val);
            }
            Tvalue::Int16(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::I16]);
                    buffer.write_i16(id);
                }
                buffer.write_i16(val);
            }
            Tvalue::Int32(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::I32]);
                    buffer.write_i16(id);
                }
                buffer.write_i32(val);
            }
            Tvalue::Int64(val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::I64]);
                    buffer.write_i16(id);
                }
                buffer.write_i64(val);
            }
            Tvalue::String(ref val) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::STRING]);
                    buffer.write_i16(id);
                }
                buffer.write_str(val);
            }
            Tvalue::Struct => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::STRUCT]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type struct must have an id");
                }
            }
            Tvalue::Map => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::MAP]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type map must have an id");
                }
            }
            Tvalue::Set => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::SET]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type set must have an id");
                }
            }
            Tvalue::List(ref ttype, len) => {
                if let Some(id) = p.id {
                    buffer.write_bytes(&[consts::LIST]);
                    buffer.write_i16(id);
                } else {
                    panic!("parameters of type list must have an id");
                }

                // TODO: this could be better
                let byte = match ttype.as_str() {
                    "string" => consts::STRING,
                    "struct" => consts::STRUCT,
                    _ => panic!("unsupported ttype for list"),
                };
                buffer.write_bytes(&[byte]);
                buffer.write_i32(len);
            }
        }
    }
    buffer.stop();
    buffer.frame();
    buffer.into_vec()
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::super::testutil;
    use super::*;
    #[cfg(feature = "unstable")]
    use test;

    #[test]
    fn test_ping() {
        assert_eq!(
            ping(),
            [0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0,]
        );
    }

    #[test]
    fn thrift_ping() {
        assert_eq!(
            generic("ping", 0, &mut Vec::new()),
            vec![
                0, 0, 0, 17, 128, 1, 0, 1, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0,
            ]
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn ping_benchmark(b: &mut test::Bencher) {
        b.iter(|| generic("ping", 0, &mut Vec::new()));
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn add_benchmark(b: &mut test::Bencher) {
        let mut ps = vec![
            testutil::mk_param(1, Tvalue::Int32(1)),
            testutil::mk_param(2, Tvalue::Int32(1)),
        ];
        b.iter(|| generic("add", 0, &mut ps));
    }

}
