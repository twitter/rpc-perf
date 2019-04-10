//  Copyright 2019 Twitter, Inc
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

mod simple;

pub use self::simple::Simple as SimpleHeatmap;

use time::Tm;

const SECOND: u64 = 1_000_000_000;

pub struct Slice {
    begin_utc: Tm,
    end_utc: Tm,
    begin_precise: usize,
    end_precise: usize,
    histogram: crate::histogram::latched::Histogram,
}

impl Slice {
    pub fn begin_utc(&self) -> Tm {
        self.begin_utc
    }

    pub fn end_utc(&self) -> Tm {
        self.end_utc
    }

    pub fn begin_precise(&self) -> usize {
        self.begin_precise
    }

    pub fn end_precise(&self) -> usize {
        self.end_precise
    }

    pub fn histogram(&self) -> &crate::histogram::latched::Histogram {
        &self.histogram
    }
}

pub trait Heatmap {
    fn incr(&self, time: usize, value: usize, count: usize);
    fn decr(&self, time: usize, value: usize, count: usize);
    fn latch(&self);
    fn samples(&self) -> usize;
    fn highest_count(&self) -> usize;
    fn slices(&self) -> usize;
    fn buckets(&self) -> usize;
    fn begin_utc(&self) -> Tm;
    fn begin_precise(&self) -> usize;
    fn resolution(&self) -> usize;
}

pub struct Builder {
    min: usize,
    max: usize,
    precision: usize,
    resolution: usize,
    span: usize,
}

impl Builder {
    pub fn new(min: usize, max: usize, precision: usize, resolution: usize, span: usize) -> Self {
        Self {
            min,
            max,
            precision,
            resolution,
            span,
        }
    }

    pub fn build(&self) -> Box<Heatmap> {
        Box::new(self::SimpleHeatmap::new(self.max, self.precision, self.resolution, self.span))
    }
}
