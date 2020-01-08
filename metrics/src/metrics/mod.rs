// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;
use std::collections::HashMap;

use chashmap::CHashMap;
use datastructures::*;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// The general structure which holds data and is used to add channels and their
/// outputs, record measurements, and produce readings
#[derive(Clone)]
pub struct Metrics<T: 'static>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    labels: Arc<Mutex<HashSet<String>>>,
    data: Arc<CHashMap<String, Arc<Channel<T>>>>,
}

impl<T> Metrics<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    pub fn new() -> Self {
        Self {
            labels: Arc::new(Mutex::new(HashSet::new())),
            data: Arc::new(CHashMap::new()),
        }
    }

    pub fn record_counter(&self, statistic: &dyn Statistic, time: u64, value: u64) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_counter(time, value);
        }
    }

    pub fn record_gauge(&self, statistic: &dyn Statistic, time: u64, value: u64) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_gauge(time, value);
        }
    }

    pub fn record_distribution(&self, statistic: &dyn Statistic, time: u64, value: u64, count: <T as AtomicPrimitive>::Primitive) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_distribution(time, value, count);
        }
    }

    pub fn record_delta(&self, statistic: &dyn Statistic, time: u64, value: u64) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_delta(time, value);
        }
    }

    pub fn record_increment(&self, statistic: &dyn Statistic, time: u64, count: <T as AtomicPrimitive>::Primitive) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_increment(time, count);
        }
    }

    pub fn record_time_interval(&self, statistic: &dyn Statistic, start: u64, stop: u64) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.record_time_interval(start, stop);
        }
    }

    pub fn description(&self, statistic: &dyn Statistic) -> Option<String> {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.description()
        } else {
            None
        }
    }

    pub fn unit(&self, statistic: &dyn Statistic) -> Option<String> {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.unit()
        } else {
            None
        }
    }

    pub fn reading(&self, statistic: &dyn Statistic) -> Option<u64> {
        if let Some(channel) = self.data.get(statistic.name()) {
            Some(channel.reading())
        } else {
            None
        }
    }

    pub fn percentile(&self, statistic: &dyn Statistic, percentile: f64) -> Option<u64> {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.percentile(percentile)
        } else {
            None
        }
    }

    pub fn register(&self, statistic: &dyn Statistic, summary: Option<Summary>) {
        if !self.data.contains_key(statistic.name()) {
            let mut labels = self.labels.lock().unwrap();
            labels.insert(statistic.name().to_string());
            let channel = Channel::new(statistic, summary);
            self.data
                .insert(statistic.name().to_string(), Arc::new(channel));
        }
    }

    pub fn deregister(&self, statistic: &dyn Statistic) {
        if self.data.contains_key(statistic.name()) {
            let mut labels = self.labels.lock().unwrap();
            labels.remove(statistic.name());
            self.data.remove(statistic.name());
        }
    }

    pub fn readings(&self) -> Vec<Reading> {
        let mut result = Vec::new();
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            if let Some(channel) = self.data.get(label) {
                result.extend(channel.readings());
            }
        }
        result
    }

    pub fn hash_map(&self) -> HashMap<String, HashMap<Output, u64>> {
        let mut result = HashMap::new();
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            if let Some(channel) = self.data.get(label) {
                result.insert(label.to_string(), channel.hash_map());
            }
        }
        result
    }

    #[cfg(feature = "waterfall")]
    pub fn save_files(&self) {
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            if let Some(channel) = self.data.get(label) {
                channel.save_files();
            }
        }
    }

    pub fn register_output(&self, statistic: &dyn Statistic, output: Output) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.add_output(output);
        }
    }

    pub fn deregister_output(&self, statistic: &dyn Statistic, output: Output) {
        if let Some(channel) = self.data.get(statistic.name()) {
            channel.delete_output(output);
        }
    }

    pub fn latch(&self) {
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            if let Some(channel) = self.data.get(label) {
                channel.latch();
            }
        }
    }

    pub fn zero(&self) {
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            if let Some(channel) = self.data.get(label) {
                channel.zero();
            }
        }
    }

    pub fn clear(&self) {
        let mut labels = self.labels.lock().unwrap();
        labels.clear();
        self.data.clear();
    }

    pub fn shrink_to_fit(&self) {
        self.data.shrink_to_fit();
    }
}

impl<T> Default for Metrics<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    fn default() -> Self {
        Self::new()
    }
}
