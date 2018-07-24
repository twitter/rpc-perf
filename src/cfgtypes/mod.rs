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

use std::collections::BTreeMap;
use std::sync::Arc;
pub use toml::de::Deserializer;
pub use toml::Value;

pub mod tools;

pub type CResult<T> = Result<T, String>;

pub struct BenchmarkWorkload {
    pub name: String,
    pub rate: usize,
    pub gen: Box<ProtocolGen>,
}

impl BenchmarkWorkload {
    pub fn new(name: String, rate: usize, gen: Box<ProtocolGen>) -> BenchmarkWorkload {
        BenchmarkWorkload {
            name: name,
            rate: rate,
            gen: gen,
        }
    }
}

/// Protocol specific parsing generator and workloads
pub struct ProtocolConfig {
    pub protocol: Arc<ProtocolParseFactory>,
    pub workloads: Vec<BenchmarkWorkload>,
}

#[derive(PartialEq, Debug)]
pub enum ParsedResponse {
    Error(String),
    Hit,
    Incomplete,
    Invalid,
    Miss,
    Ok,
    Unknown,
    Version(String),
}

/// Factory of protocol message buffers
pub trait ProtocolGen: Send {
    /// Generate the next buffer to send to the server
    fn generate_message(&mut self) -> Vec<u8>;

    /// The method being called on the server
    fn method(&self) -> &str;
}

/// Factory for `ProtocolParse` instances
pub trait ProtocolParseFactory: Send + Sync {
    /// Create a new protocol parser
    fn new(&self) -> Box<ProtocolParse>;

    /// Name of this protocol
    fn name(&self) -> &str;

    /// Generate some preparatory messages for the work queue
    fn prepare(&self) -> CResult<Vec<Vec<u8>>> {
        Ok(Vec::new())
    }
}

/// Protocol specific parser
pub trait ProtocolParse: Send {
    /// Parse the response buffer
    fn parse(&self, bytes: &[u8]) -> ParsedResponse;
}

/// Reusable paramter type with parser
pub trait Ptype: Sized {
    /// generate new state
    fn regen(&mut self);
    /// parse a `Ptype` from a toml tree
    fn parse(seed: usize, size: usize, num: u64, table: &BTreeMap<String, Value>) -> CResult<Self>;
}

/// `Parameter` generation style
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Style {
    /// Values are generated based on a seed
    Static,
    /// Values are generated randomly
    Random,
}

#[derive(Clone, Debug)]
pub struct Parameter<T: Ptype> {
    /// initialization `Style` of the parameter
    pub style: Style,
    /// whether the parameter should be regenerated on each use
    pub regenerate: bool,
    /// current value of the `Parameter`
    pub value: T,
}

impl<T: Default + Ptype> Default for Parameter<T> {
    fn default() -> Parameter<T> {
        Parameter {
            style: Style::Static,
            regenerate: false,
            value: T::default(),
        }
    }
}

impl<T: Ptype> Parameter<T> {
    /// Mutate internal value if necessary
    ///
    /// If the parameter is flagged to not regenerate or was seeded, it isn't regenerated.
    pub fn regen(&mut self) {
        if self.regenerate && self.style == Style::Random {
            self.value.regen()
        }
    }
}

/// Extract a `Parameter` from the toml tree
pub fn extract_parameter<T: Ptype>(
    index: usize,
    parameter: &BTreeMap<String, Value>,
) -> CResult<Parameter<T>> {
    let style = match parameter.get("style").and_then(|k| k.as_str()) {
        Some("random") => Style::Random,
        Some("static") | None => Style::Static,
        Some(other) => return Err(format!("bad parameter style: {}", other)),
    };

    let seed = parameter
        .get("seed")
        .and_then(|k| k.as_integer())
        .map_or(index, |i| i as usize);

    let size = parameter
        .get("size")
        .and_then(|k| k.as_integer())
        .map_or(1, |i| i as usize);

    let num = parameter.get("num").and_then(|k| k.as_integer()).map_or(
        0,
        |i| {
            i as u64
        },
    );

    // size is insufficient to contain num strings
    if format!("{}", num - 1).len() > size {
        return Err(format!("size {} insufficient to contain {} strings",
                           size, num));
    }

    let regenerate = parameter
        .get("regenerate")
        .and_then(|k| k.as_bool())
        .unwrap_or(false);

    let mut value = try!(T::parse(seed, size, num, parameter));

    // initialize with a random value if that is what is needed
    if style == Style::Random {
        value.regen();
    }

    Ok(Parameter {
        style: style,
        regenerate: regenerate,
        value: value,
    })
}
