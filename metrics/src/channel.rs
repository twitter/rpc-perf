// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;

use datastructures::*;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
/// Measurements are point-in-time readings taken from some datasource. They are
/// sent to the metrics library for aggregation and exposition.
pub enum Measurement<C> {
    // taken from a counter eg: number of requests
    Counter { value: u64, time: u64 },
    // taken from a distribution eg: an external histogram
    Distribution { value: u64, count: C, time: u64 },
    // taken from a gauge eg: bytes of memory used
    Gauge { value: u64, time: u64 },
    // incremental count to sum into a counter
    Increment { count: C, time: u64 },
    // the start and stop of an event
    TimeInterval { start: u64, stop: u64 },
}

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

/// A channel tracks measurements that are taken from the same datasource. For
/// example, you might use a channel to track requests and another for CPU
/// utilization.
pub struct Channel<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
{
    name: Arc<Mutex<String>>,
    source: Source,
    counter: AtomicU64,
    histogram: Option<Histogram<T>>,
    last_write: AtomicU64,
    latched: bool,
    max: Point,
    min: Point,
    outputs: Arc<Mutex<HashSet<Output>>>,
    has_data: AtomicBool,
}

impl<T: 'static> PartialEq for Channel<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    fn eq(&self, other: &Channel<T>) -> bool {
        self.name() == other.name()
    }
}

impl<T: 'static> Eq for Channel<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
}

impl<T: 'static> Channel<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    /// Create a new channel with a given name, source, and an optional
    /// histogram which can be used to generate percentile metrics
    pub fn new(name: String, source: Source, histogram: Option<Histogram<T>>) -> Self {
        Self {
            name: Arc::new(Mutex::new(name)),
            source,
            counter: AtomicU64::default(),
            histogram,
            last_write: AtomicU64::default(),
            latched: true,
            max: Point::new(0, 0),
            min: Point::new(0, 0),
            outputs: Arc::new(Mutex::new(HashSet::new())),
            has_data: AtomicBool::new(false),
        }
    }

    /// Return the name of the `Channel`
    pub fn name(&self) -> String {
        self.name.lock().unwrap().clone()
    }

    /// Return the source of the `Channel`
    pub fn source(&self) -> Source {
        self.source
    }

    /// Record a new measurement for the `Channel`
    pub fn record(&self, measurement: Measurement<<T as AtomicPrimitive>::Primitive>) {
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
            Measurement::Increment { count, time } => self.record_increment(count, time),
            Measurement::TimeInterval { start, stop } => self.record_time_interval(start, stop),
        }
    }

    // for Counter measurements:
    // counter tracks value
    // histogram tracks rate of change
    fn record_counter(&self, value: u64, time: u64) {
        if self.source == Source::Counter {
            if self.has_data.load(Ordering::SeqCst) {
                // calculate the difference between consecutive readings and the rate
                let delta_value = value.wrapping_sub(self.counter.get());
                let delta_time = time.wrapping_sub(self.last_write.get());
                let rate = (delta_value as f64 * (1_000_000_000.0 / delta_time as f64)) as u64;
                self.counter.add(delta_value);
                if let Some(ref histogram) = self.histogram {
                    histogram.increment(rate, <T as AtomicPrimitive>::Primitive::from(1_u8));
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
                self.has_data.store(true, Ordering::SeqCst);
            }
            self.last_write.set(time);
        }
    }

    // for Distribution measurements:
    // counter tracks sum of all counts
    // histogram tracks values
    fn record_distribution(&self, value: u64, count: <T as AtomicPrimitive>::Primitive, time: u64) {
        if self.source == Source::Distribution {
            self.counter.add(u64::from(count));
            if let Some(ref histogram) = self.histogram {
                histogram.increment(value, count);
            }
            self.last_write.set(time);
        }
    }

    // for Gauge measurements:
    // counter tracks latest reading
    // histogram tracks readings
    // max tracks largest reading
    // min tracks smallest reading
    fn record_gauge(&self, value: u64, time: u64) {
        if self.source == Source::Gauge {
            self.counter.set(value);
            if let Some(ref histogram) = self.histogram {
                histogram.increment(value, <T as AtomicPrimitive>::Primitive::from(1_u8));
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
    fn record_increment(&self, count: <T as AtomicPrimitive>::Primitive, time: u64) {
        if self.source == Source::Counter {
            self.counter.add(u64::from(count));
            if let Some(ref histogram) = self.histogram {
                histogram.increment(
                    u64::from(count),
                    <T as AtomicPrimitive>::Primitive::from(1_u8),
                );
            }
            self.last_write.set(time);
        }
    }

    // for TimeInterval measurements, we increment the histogram with duration of event
    fn record_time_interval(&self, start: u64, stop: u64) {
        if self.source == Source::TimeInterval {
            self.counter.add(1);
            let duration = stop - start;
            if let Some(ref histogram) = self.histogram {
                histogram.increment(duration, <T as AtomicPrimitive>::Primitive::from(1_u8));
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

    /// Get the counter from the `Channel`
    pub fn counter(&self) -> u64 {
        self.counter.get()
    }

    /// Calculate a percentile from the histogram, returns `None` if there is no
    /// histogram for the `Channel`
    pub fn percentile(&self, percentile: f64) -> Option<u64> {
        if let Some(ref histogram) = self.histogram {
            histogram.percentile(percentile)
        } else {
            None
        }
    }

    /// Register an `Output` for exposition
    pub fn add_output(&self, output: Output) {
        let mut outputs = self.outputs.lock().unwrap();
        outputs.insert(output);
    }

    /// Remove an `Output` from exposition
    pub fn delete_output(&self, output: Output) {
        let mut outputs = self.outputs.lock().unwrap();
        outputs.remove(&output);
    }

    /// Resets any latched aggregators, `Histograms` may be latched or windowed.
    /// Min and max value-time tracking are currently always latched and need to
    /// be reset using this function.
    pub fn latch(&self) {
        if self.latched {
            if let Some(ref histogram) = self.histogram {
                histogram.clear();
            }
        }
        self.max.set(0, 0);
        self.min.set(0, 0);
    }

    /// Zeros out all stored data for the `Channel`
    pub fn zero(&self) {
        self.has_data.store(false, Ordering::SeqCst);
        self.last_write.set(0);
        self.counter.set(0);
        if let Some(ref histogram) = self.histogram {
            histogram.clear();
        }
        self.max.set(0, 0);
        self.min.set(0, 0);
    }

    /// Calculates the total set of `Readings` that are produced based on the
    /// `Outputs` which have been added for the `Channel`
    pub fn readings(&self) -> Vec<Reading> {
        let mut result = Vec::new();
        let outputs = self.outputs.lock().unwrap();
        for output in &*outputs {
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
        result
    }

    /// Calculates and returns the `Output`s with their values as a `HashMap`
    pub fn hash_map(&self) -> HashMap<Output, u64> {
        let mut result = HashMap::new();
        let outputs = self.outputs.lock().unwrap();
        for output in &*outputs {
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
        result
    }
}
