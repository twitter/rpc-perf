// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[derive(PartialEq, Debug, Copy, Clone)]
/// A `Source` defines what type of datasource a measurement is taken from
pub enum Source {
    /// Used for free-running counters
    Counter,
    /// Taken from a histogram
    Distribution,
    /// Taken from a gauge which may increase or decrease between readings
    Gauge,
    /// Start and stop times from discrete events
    TimeInterval,
}
