// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
/// Percentiles are used to specify the exported percentiles from histograms
pub enum Percentile {
    Minimum,
    p001,
    p01,
    p1,
    p5,
    p10,
    p25,
    p50,
    p75,
    p90,
    p95,
    p99,
    p999,
    p9999,
    Maximum,
}

impl Percentile {
    pub fn as_f64(self) -> f64 {
        match self {
            Percentile::Minimum => 0.0,
            Percentile::p001 => 0.0001,
            Percentile::p01 => 0.001,
            Percentile::p1 => 0.01,
            Percentile::p5 => 0.05,
            Percentile::p10 => 0.10,
            Percentile::p25 => 0.25,
            Percentile::p50 => 0.5,
            Percentile::p75 => 0.75,
            Percentile::p90 => 0.9,
            Percentile::p95 => 0.95,
            Percentile::p99 => 0.99,
            Percentile::p999 => 0.999,
            Percentile::p9999 => 0.9999,
            Percentile::Maximum => 1.0,
        }
    }
}

impl std::fmt::Display for Percentile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Percentile::Minimum => write!(f, "minimum"),
            Percentile::p001 => write!(f, "p001"),
            Percentile::p01 => write!(f, "p01"),
            Percentile::p1 => write!(f, "p1"),
            Percentile::p5 => write!(f, "p5"),
            Percentile::p10 => write!(f, "p10"),
            Percentile::p25 => write!(f, "p25"),
            Percentile::p50 => write!(f, "p50"),
            Percentile::p75 => write!(f, "p75"),
            Percentile::p90 => write!(f, "p90"),
            Percentile::p95 => write!(f, "p95"),
            Percentile::p99 => write!(f, "p99"),
            Percentile::p999 => write!(f, "p999"),
            Percentile::p9999 => write!(f, "p9999"),
            Percentile::Maximum => write!(f, "maximum"),
        }
    }
}
