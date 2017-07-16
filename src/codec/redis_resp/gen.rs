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

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[cfg(feature = "unstable")]
    #[allow(unused_imports)]
    use test;

    #[test]
    fn test_flushall() {
        assert_eq!(flushall(), "*1\r\n$8\r\nflushall\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn flushall_benchmark(b: &mut test::Bencher) {
        b.iter(|| flushall());
    }

    #[test]
    fn test_select() {
        assert_eq!(select(&1), "*2\r\n$6\r\nselect\r\n$1\r\n1\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn select_benchmark(b: &mut test::Bencher) {
        b.iter(|| select(&1));
    }

    #[test]
    fn test_set() {
        assert_eq!(
            set("key", "value"),
            "*3\r\n$3\r\nset\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn set_benchmark(b: &mut test::Bencher) {
        b.iter(|| set("key", "value"));
    }

    #[test]
    fn test_hset() {
        assert_eq!(
            hset("hash", "key", "value"),
            "*4\r\n$4\r\nhset\r\n$4\r\nhash\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn hset_benchmark(b: &mut test::Bencher) {
        b.iter(|| hset("hash", "key", "value"));
    }

    #[test]
    fn test_get() {
        assert_eq!(get("key"), "*2\r\n$3\r\nget\r\n$3\r\nkey\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn get_benchmark(b: &mut test::Bencher) {
        b.iter(|| get("key"));
    }

    #[test]
    fn test_hget() {
        assert_eq!(
            hget("hash", "key"),
            "*3\r\n$4\r\nhget\r\n$4\r\nhash\r\n$3\r\nkey\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn hget_benchmark(b: &mut test::Bencher) {
        b.iter(|| hget("hash", "key"));
    }

    #[test]
    fn test_del() {
        assert_eq!(del("key"), "*2\r\n$3\r\ndel\r\n$3\r\nkey\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn del_benchmark(b: &mut test::Bencher) {
        b.iter(|| del("key"));
    }

    #[test]
    fn test_incr() {
        assert_eq!(incr("key"), "*2\r\n$4\r\nincr\r\n$3\r\nkey\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn incr_benchmark(b: &mut test::Bencher) {
        b.iter(|| incr("key"));
    }

    #[test]
    fn test_decr() {
        assert_eq!(decr("key"), "*2\r\n$4\r\ndecr\r\n$3\r\nkey\r\n");
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn decr_benchmark(b: &mut test::Bencher) {
        b.iter(|| decr("key"));
    }

    #[test]
    fn test_expire() {
        assert_eq!(
            expire("key", 1000),
            "*3\r\n$6\r\nexpire\r\n$3\r\nkey\r\n$4\r\n1000\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn expire_benchmark(b: &mut test::Bencher) {
        b.iter(|| expire("key", 1000));
    }

    #[test]
    fn test_append() {
        assert_eq!(
            append("key", "value"),
            "*3\r\n$6\r\nappend\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn append_benchmark(b: &mut test::Bencher) {
        b.iter(|| append("key", "value"));
    }

    #[test]
    fn test_prepend() {
        assert_eq!(
            prepend("key", "value"),
            "*3\r\n$7\r\nprepend\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn prepend_benchmark(b: &mut test::Bencher) {
        b.iter(|| prepend("key", "value"));
    }

}

/// FLUSHALL request
pub fn flushall() -> String {
    "*1\r\n$8\r\nflushall\r\n".to_owned()
}

/// SELECT request
pub fn select(database: &u32) -> String {
    let database = format!("{}", database);
    format!(
        "*2\r\n$6\r\nselect\r\n${}\r\n{}\r\n",
        database.len(),
        database
    )
}

/// SET request
pub fn set(key: &str, value: &str) -> String {
    format!(
        "*3\r\n$3\r\nset\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        key.len(),
        key,
        value.len(),
        value
    )
}

/// HSET request
pub fn hset(hash: &str, key: &str, value: &str) -> String {
    format!(
        "*4\r\n$4\r\nhset\r\n${}\r\n{}\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        hash.len(),
        hash,
        key.len(),
        key,
        value.len(),
        value
    )
}

/// GET request
pub fn get(key: &str) -> String {
    format!("*2\r\n$3\r\nget\r\n${}\r\n{}\r\n", key.len(), key)
}

/// HGET request
pub fn hget(hash: &str, key: &str) -> String {
    format!(
        "*3\r\n$4\r\nhget\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        hash.len(),
        hash,
        key.len(),
        key
    )
}

/// DEL request
pub fn del(key: &str) -> String {
    format!("*2\r\n$3\r\ndel\r\n${}\r\n{}\r\n", key.len(), key)
}

/// INCR request
pub fn incr(key: &str) -> String {
    format!("*2\r\n$4\r\nincr\r\n${}\r\n{}\r\n", key.len(), key)
}

/// DECR request
pub fn decr(key: &str) -> String {
    format!("*2\r\n$4\r\ndecr\r\n${}\r\n{}\r\n", key.len(), key)
}

/// EXPIRE request
pub fn expire(key: &str, ttl: u32) -> String {
    let ttl = format!("{}", ttl);
    format!(
        "*3\r\n$6\r\nexpire\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        key.len(),
        key,
        ttl.len(),
        ttl
    )
}

/// APPEND request
pub fn append(key: &str, value: &str) -> String {
    format!(
        "*3\r\n$6\r\nappend\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        key.len(),
        key,
        value.len(),
        value
    )
}

/// PREPEND request
pub fn prepend(key: &str, value: &str) -> String {
    format!(
        "*3\r\n$7\r\nprepend\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
        key.len(),
        key,
        value.len(),
        value
    )
}
