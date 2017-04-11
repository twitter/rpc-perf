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

#![cfg_attr(feature = "unstable", feature(test))]

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[cfg(feature = "unstable")]
    use test;


    #[test]
    fn test_flushall() {
        assert_eq!(flushall(), "flushall\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn flushall_benchmark(b: &mut test::Bencher) {
        b.iter(|| flushall());
    }

    #[test]
    fn test_set() {
        assert_eq!(set("key", "value"), "set key value\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn set_benchmark(b: &mut test::Bencher) {
        b.iter(|| set("key", "value"));
    }

    #[test]
    fn test_hset() {
        assert_eq!(hset("hash", "key", "value"), "hset hash key value\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn hset_benchmark(b: &mut test::Bencher) {
        b.iter(|| hset("hash", "key", "value"));
    }
    #[test]
    fn test_get() {
        assert_eq!(get("key"), "get key\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn get_benchmark(b: &mut test::Bencher) {
        b.iter(|| get("key"));
    }


    #[test]
    fn test_hget() {
        assert_eq!(hget("hash", "key"), "hget hash key\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn hget_benchmark(b: &mut test::Bencher) {
        b.iter(|| hget("hash", "key"));
    }

    #[test]
    fn test_del() {
        assert_eq!(del("key"), "del key\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn del_benchmark(b: &mut test::Bencher) {
        b.iter(|| del("key"));
    }

    #[test]
    fn test_expire() {
        assert_eq!(expire("key", 1000), "expire key 1000\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn expire_benchmark(b: &mut test::Bencher) {
        b.iter(|| expire("key", 1000));
    }

    #[test]
    fn test_incr() {
        assert_eq!(incr("key"), "incr key\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn incr_benchmark(b: &mut test::Bencher) {
        b.iter(|| incr("key"));
    }

    #[test]
    fn test_decr() {
        assert_eq!(decr("key"), "decr key\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn decr_benchmark(b: &mut test::Bencher) {
        b.iter(|| decr("key"));
    }

    #[test]
    fn test_append() {
        assert_eq!(append("key", "value"), "append key value\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn append_benchmark(b: &mut test::Bencher) {
        b.iter(|| append("key", "value"));
    }

    #[test]
    fn test_prepend() {
        assert_eq!(prepend("key", "value"), "prepend key value\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn prepend_benchmark(b: &mut test::Bencher) {
        b.iter(|| prepend("key", "value"));
    }

}

/// FLUSHALL request
pub fn flushall() -> String {
    "flushall\r\n".to_owned()
}

/// SELECT request
pub fn select(database: &u32) -> String {
    format!("select {}\r\n", database)
}

/// SET request
pub fn set(key: &str, value: &str) -> String {
    format!("set {} {}\r\n", key, value)
}

/// HSET request
pub fn hset(hash: &str, key: &str, value: &str) -> String {
    format!("hset {} {} {}\r\n", hash, key, value)
}

/// GET request
pub fn get(key: &str) -> String {
    format!("get {}\r\n", key)
}

/// HGET request
pub fn hget(hash: &str, key: &str) -> String {
    format!("hget {} {}\r\n", hash, key)
}

/// DEL request
pub fn del(key: &str) -> String {
    format!("del {}\r\n", key)
}

/// EXPIRE request
pub fn expire(key: &str, seconds: usize) -> String {
    format!("expire {} {}\r\n", key, seconds)
}

/// INCR request
pub fn incr(key: &str) -> String {
    format!("incr {}\r\n", key)
}

/// DECR request
pub fn decr(key: &str) -> String {
    format!("decr {}\r\n", key)
}

/// APPEND request
pub fn append(key: &str, value: &str) -> String {
    format!("append {} {}\r\n", key, value)
}

/// PREPEND request
pub fn prepend(key: &str, value: &str) -> String {
    format!("prepend {} {}\r\n", key, value)
}
