#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
#[macro_use]
extern crate log;
extern crate bytes;
extern crate byteorder;
extern crate crc;
extern crate getopts;
extern crate mio;
extern crate mpmc;
extern crate pad;
extern crate time;
extern crate rand;
extern crate ratelimit;
extern crate slab;
extern crate tic;
extern crate toml;

use std::str;

#[path = "../../src/cfgtypes/mod.rs"]
mod cfgtypes;

#[path = "../../src/codec/mod.rs"]
mod codec;

use cfgtypes::*;
use codec::echo::EchoParser;

//ProtocolParseFactory
fuzz_target!(|data: &[u8]| {
                 let parser = EchoParser;
                 let _ = parser.parse(data);
             });
