// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[derive(Debug)]
/// Measurements are point-in-time readings taken from some datasource. They are
/// sent to the metrics library for aggregation and exposition.
pub enum Measurement<C> {
    // taken from a counter eg: number of requests
    Counter { value: u64, time: u64 },
    // increment a counter eg: increment by number of bytes transmitted
    Delta { value: u64, time: u64 },
    // taken from a distribution eg: an external histogram
    Distribution { value: u64, count: C, time: u64 },
    // taken from a gauge eg: bytes of memory used
    Gauge { value: u64, time: u64 },
    // incremental count to sum into a counter
    Increment { count: C, time: u64 },
    // the start and stop of an event
    TimeInterval { start: u64, stop: u64 },
}
