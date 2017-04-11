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

use pad::{Alignment, PadStr};
use rand::{Rng, thread_rng};


pub fn random_string(size: usize) -> String {
    thread_rng().gen_ascii_chars().take(size).collect()
}

pub fn random_bytes(size: usize) -> Vec<u8> {
    random_string(size).into_bytes()
}

pub fn seeded_string(size: usize, seed: usize) -> String {
    let s = format!("{}", seed);
    s.pad(size, '0', Alignment::Right, true)
}
