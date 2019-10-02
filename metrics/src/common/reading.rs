// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Output;

/// Readings are a value for an output type for a given channel
pub struct Reading {
    label: String,
    output: Output,
    value: u64,
}

impl Reading {
    /// Creates a new reading from its fields
    pub fn new(label: String, output: Output, value: u64) -> Self {
        Self {
            label,
            output,
            value,
        }
    }

    /// Returns the output type
    pub fn output(&self) -> Output {
        self.output.clone()
    }

    /// Returns the label `Label` of the source
    pub fn label(&self) -> String {
        self.label.clone()
    }

    /// Returns the value for the `Output`
    pub fn value(&self) -> u64 {
        self.value
    }
}
