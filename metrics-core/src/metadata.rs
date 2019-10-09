// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::ops::Index;

const EMPTY_ARRAY: &[(&str, &str)] = &[];

/// A static map of key-value pairs.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    attributes: &'static [(&'static str, &'static str)],
}

impl Metadata {
    // Note: This is not public since in the future we may want to enforce that
    // these are sorted
    pub(crate) fn new(attributes: &'static [(&'static str, &'static str)]) -> Self {
        Self { attributes }
    }

    /// Get an iterator over the key-value pairs stored in this `Metadata`
    /// instance.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.attributes.iter().copied()
    }

    /// Get the value for the metadata key.
    pub fn get(&self, val: &str) -> Option<&'static str> {
        // TODO(sean): If we can guarantee that the attributes are always sorted
        // then we can use a binary search here instead.
        for (k, v) in self.iter() {
            if k == val {
                return Some(v);
            }
        }

        None
    }

    /// Create an empty set of metadata
    pub const fn empty() -> Self {
        Self {
            attributes: EMPTY_ARRAY,
        }
    }
}

impl Index<&'_ str> for Metadata {
    type Output = str;

    fn index(&self, key: &str) -> &'static str {
        match self.get(key) {
            Some(x) => x,
            None => panic!("key `{}` not within the metadata map", key),
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata::empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_metadata() {
        let data = metadata! {
            "str" => "x",
            "a" => "b",
            "units" => "ms",
            "status" => "is a test",
        };

        assert_eq!(&data["a"], "b");
        assert_eq!(&data["units"], "ms");
        assert_eq!(&data["status"], "is a test");
        assert_eq!(&data["str"], "x");
        assert_eq!(data.get("status_"), None);
        assert_eq!(data.get("not present"), None);
    }

    #[test]
    fn struct_style_metadata() {
        let data = metadata! {
            unit: "ms",
            status: "dead",
            test: "true"
        };

        assert_eq!(&data["unit"], "ms");
        assert_eq!(&data["status"], "dead");
        assert_eq!(&data["test"], "true");
        assert_eq!(data.get("u"), None);
        assert_eq!(data.get(""), None);
    }
}
