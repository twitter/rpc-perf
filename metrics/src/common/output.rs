// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Percentile;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Outputs are used to specify the metrics exported for a channel
pub enum Output {
    Reading,
    MaxPointTime,
    MinPointTime,
    Percentile(Percentile),
}
