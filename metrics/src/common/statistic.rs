// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Source;

pub trait Statistic {
    /// the reported name of the series
    fn name(&self) -> &str;

    /// the unit of measurement
    fn unit(&self) -> Option<&str> {
        None
    }

    /// describe the meaning of the statistic
    fn description(&self) -> Option<&str> {
        None
    }

    /// the source of the measurement
    fn source(&self) -> Source;
}
