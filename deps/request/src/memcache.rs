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

pub use super::*;

/// request to change memcache verbosity
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(verbosity(4), "verbosity 4\r\n");
pub fn verbosity(level: usize) -> String {
    format!("verbosity {}\r\n", level)
}

/// create a memcache version tcp request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(version(), "version\r\n");
pub fn version() -> String {
    format!("version\r\n")
}

/// create a memcache quit request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(quit(), "quit\r\n");
pub fn quit() -> String {
    format!("quit\r\n")
}

/// create a set request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(set("key", "value", None, None), "set key 0 0 5\r\nvalue\r\n");
pub fn set(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("set {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            value)
}

/// create a cas request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(cas("key", "value", None, None, 100_u64), "cas key 0 0 5 100\r\nvalue\r\n");
pub fn cas(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>, cas: u64) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("cas {} {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            cas,
            value)
}

/// create an add request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(add("key", "value", None, None), "add key 0 0 5\r\nvalue\r\n");
pub fn add(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("add {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            value)
}

/// create a replace request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(replace("key", "value", None, None), "replace key 0 0 5\r\nvalue\r\n");
pub fn replace(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("replace {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            value)
}

/// create an append request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(append("key", "value", None, None), "append key 0 0 5\r\nvalue\r\n");
pub fn append(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("append {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            value)
}

/// create an prepend request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(prepend("key", "value", None, None), "prepend key 0 0 5\r\nvalue\r\n");
pub fn prepend(key: &str, value: &str, exptime: Option<u32>, flags: Option<u32>) -> String {
    let flags = flags.unwrap_or(0);
    let exptime = exptime.unwrap_or(0);
    format!("prepend {} {} {} {}\r\n{}\r\n",
            key,
            flags,
            exptime,
            value.len(),
            value)
}

/// create a incr request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(incr("key", 1), "incr key 1\r\n");
/// assert_eq!(incr("key", 1000), "incr key 1000\r\n");
pub fn incr(key: &str, value: u64) -> String {
    format!("incr {} {}\r\n", key, value)
}

/// create a decr request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(decr("key", 1), "decr key 1\r\n");
/// assert_eq!(decr("key", 1000), "decr key 1000\r\n");
pub fn decr(key: &str, value: u64) -> String {
    format!("decr {} {}\r\n", key, value)
}

/// create a touch request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(touch("key", None), "touch key 0\r\n");
pub fn touch(key: &str, exptime: Option<u32>) -> String {
    let exptime = exptime.unwrap_or(0);
    format!("touch {} {}\r\n", key, exptime)
}

/// create a get request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(get("key"), "get key\r\n");
pub fn get(key: &str) -> String {
    format!("get {}\r\n", key)
}

/// create a gets request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(gets("key"), "gets key\r\n");
pub fn gets(key: &str) -> String {
    format!("gets {}\r\n", key)
}

/// create a delete request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(delete("key"), "delete key\r\n");
pub fn delete(key: &str) -> String {
    format!("delete {}\r\n", key)
}

/// create a flush_all request
///
/// # Example
/// ```
/// # use request::memcache::*;
///
/// assert_eq!(flush_all(), "flush_all\r\n");
pub fn flush_all() -> String {
    format!("flush_all\r\n")
}
