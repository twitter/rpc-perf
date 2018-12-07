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

use datastructures::HistogramConfig;
use crate::*;

use datastructures::{Counter, Histogram, RwWrapper};

use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug)]
pub enum Measurement {
    // taken from a counter eg: number of requests
    Counter {
        value: usize,
        time: usize,
    },
    // taken from a distribution eg: an external histogram
    Distribution {
        value: usize,
        count: usize,
        time: usize,
    },
    // taken from a gauge eg: bytes of memory used
    Gauge {
        value: usize,
        time: usize,
    },
    // incremental count to sum into a counter
    Increment {
        value: usize,
        time: usize,
    },
    // the start and stop of an event
    TimeInterval {
        start: usize,
        stop: usize,
    },
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Source {
    Counter,
    Distribution,
    Gauge,
    TimeInterval,
}

// #[derive(Clone)]
pub struct Channel {
    name: RwWrapper<String>,
    source: Source,
    counter: Counter,
    histogram: Option<Box<Histogram>>,
    last_write: Counter,
    latched: bool,
    max: Point,
    min: Point,
    outputs: RwWrapper<HashSet<Output>>,
}

impl PartialEq for Channel {
    fn eq(&self, other: &Channel) -> bool {
        self.name() == other.name()
    }
}

impl Eq for Channel { }

impl Channel {
    pub fn new(name: String, source: Source, histogram_config: Option<HistogramConfig>) -> Self {
        let histogram = if let Some(config) = histogram_config {
            Some(config.build())
        } else {
            None
        };
        Self {
            name: RwWrapper::new(name),
            source,
            counter: Counter::default(),
            histogram,
            last_write: Counter::default(),
            latched: true,
            max: Point::new(0, 0),
            min: Point::new(0, 0),
            outputs: RwWrapper::new(HashSet::new()),
        }
    }

    pub fn name(&self) -> String {
        unsafe { (*self.name.get()).clone() }
    }

    pub fn source(&self) -> Source {
        self.source
    }

    pub fn record(&self, measurement: Measurement) {
        trace!("record: {} {:?}", self.name(), measurement);
        match measurement {
            Measurement::Counter { value, time } => {
                self.record_counter(value, time);
            }
            Measurement::Distribution { value, count, time } => {
                self.record_distribution(value, count, time);
            }
            Measurement::Gauge { value, time } => {
                self.record_gauge(value, time);
            }
            Measurement::Increment { value, time } => self.record_increment(value, time),
            Measurement::TimeInterval { start, stop } => self.record_time_interval(start, stop),
        }
    }

    // for Counter measurements:
    // counter tracks value
    // histogram tracks rate of change
    fn record_counter(&self, value: usize, time: usize) {
        if self.source == Source::Counter {
            let previous = self.counter.get();
            if previous > 0 {
                // calculate the difference between consecutive readings and the rate
                let delta_value = value - previous;
                let delta_time = time - self.last_write.get();
                let rate = (delta_value as f64 * (1_000_000_000.0 / delta_time as f64)) as usize;
                trace!(
                    "delta value: {} time: {} rate: {}",
                    delta_value,
                    delta_time,
                    rate
                );
                self.counter.incr(delta_value);
                if let Some(ref histogram) = self.histogram {
                    histogram.incr(rate, 1);
                }
                // track the point of max rate
                if self.max.time() > 0 {
                    if rate > self.max.value() {
                        self.max.set(rate, time);
                    }
                } else {
                    self.max.set(rate, time);
                }
                // track the point of min rate
                if self.min.time() > 0 {
                    if rate < self.min.value() {
                        self.min.set(rate, time);
                    }
                } else {
                    self.min.set(rate, time);
                }
            } else {
                self.counter.set(value);
            }
            self.last_write.set(time);
        }
    }

    // for Distribution measurements:
    // counter tracks sum of all counts
    // histogram tracks values
    fn record_distribution(&self, value: usize, count: usize, time: usize) {
        if self.source == Source::Distribution {
            self.counter.incr(count);
            if let Some(ref histogram) = self.histogram {
                histogram.incr(value, count);
            }
            self.last_write.set(time);
        }
    }

    // for Gauge measurements:
    // counter tracks latest reading
    // histogram tracks readings
    // max tracks largest reading
    // min tracks smallest reading
    fn record_gauge(&self, value: usize, time: usize) {
        if self.source == Source::Gauge {
            self.counter.set(value);
            if let Some(ref histogram) = self.histogram {
                histogram.incr(value, 1);
            }
            // track the point of max gauge reading
            if self.max.time() > 0 {
                if value > self.max.value() {
                    self.max.set(value, time);
                }
            } else {
                self.max.set(value, time);
            }
            // track the point of min rate
            if self.min.time() > 0 {
                if value < self.min.value() {
                    self.min.set(value, time);
                }
            } else {
                self.min.set(value, time);
            }
            self.last_write.set(time);
        }
    }

    // for Increment measurements:
    // counter tracks sum of all increments
    // histogram tracks magnitude of increments
    fn record_increment(&self, value: usize, time: usize) {
        if self.source == Source::Counter {
            self.counter.incr(value);
            if let Some(ref histogram) = self.histogram {
                histogram.incr(value, 1);
            }
            self.last_write.set(time);
        }
    }

    // for TimeInterval measurements, we increment the histogram with duration of event
    fn record_time_interval(&self, start: usize, stop: usize) {
        if self.source == Source::TimeInterval {
            self.counter.incr(1);
            let duration = stop - start;
            if let Some(ref histogram) = self.histogram {
                histogram.incr(duration, 1);
            }
            // track point of largest interval
            if self.max.time() > 0 {
                if duration > self.max.value() {
                    self.max.set(duration, start);
                }
            } else {
                self.max.set(duration, start);
            }
            // track point of smallest interval
            if self.min.time() > 0 {
                if duration < self.min.value() {
                    self.min.set(duration, start);
                }
            } else {
                self.min.set(duration, start);
            }
        }
    }

    pub fn counter(&self) -> usize {
        self.counter.get()
    }

    pub fn percentile(&self, percentile: f64) -> Option<usize> {
        if let Some(ref histogram) = self.histogram {
            histogram.percentile(percentile)
        } else {
            None
        }
    }

    pub fn add_output(&self, output: Output) {
        trace!("add output: {} {:?}", self.name(), output);
        unsafe {
            (*self.outputs.lock()).insert(output);
        }
    }

    pub fn delete_output(&self, output: Output) {
        trace!("delete output: {} {:?}", self.name(), output);
        unsafe {
            (*self.outputs.lock()).remove(&output);
        }
    }

    pub fn latch(&self) {
        if self.latched {
            if let Some(ref histogram) = self.histogram {
                histogram.clear();
            }
        }
        self.max.set(0, 0);
        self.min.set(0, 0);
    }

    pub fn clear(&self) {
        self.last_write.set(0);
        self.counter.set(0);
        if let Some(ref histogram) = self.histogram {
            histogram.clear();
        }
        self.max.set(0, 0);
        self.min.set(0, 0);
    }

    pub fn readings(&self) -> Vec<Reading> {
        let mut result = Vec::new();
        unsafe {
            for output in (*self.outputs.lock()).iter() {
                trace!("generate reading for: {} {:?}", self.name(), *output);
                match output {
                    Output::Counter => {
                        result.push(Reading::new(self.name(), output.clone(), self.counter()));
                    }
                    Output::MaxPointTime => {
                        if self.max.time() > 0 {
                            result.push(Reading::new(self.name(), output.clone(), self.max.time()));
                        }
                    }
                    Output::MinPointTime => {
                        if self.max.time() > 0 {
                            result.push(Reading::new(self.name(), output.clone(), self.min.time()));
                        }
                    }
                    Output::Percentile(percentile) => {
                        if let Some(value) = self.percentile(percentile.as_f64()) {
                            result.push(Reading::new(self.name(), output.clone(), value));
                        }
                    }
                }
            }
        }
        result
    }

    pub fn hash_map(&self) -> HashMap<Output, usize> {
        let mut result = HashMap::new();
        unsafe {
            for output in (*self.outputs.lock()).iter() {
                trace!("generate reading for: {} {:?}", self.name(), *output);
                match output {
                    Output::Counter => {
                        result.insert(output.clone(), self.counter());
                    }
                    Output::MaxPointTime => {
                        if self.max.time() > 0 {
                            result.insert(output.clone(), self.max.time());
                        }
                    }
                    Output::MinPointTime => {
                        if self.max.time() > 0 {
                            result.insert(output.clone(), self.min.time());
                        }
                    }
                    Output::Percentile(percentile) => {
                        if let Some(value) = self.percentile(percentile.as_f64()) {
                            result.insert(output.clone(), value);
                        }
                    }
                }
            }
        }
        result
    }
}
