// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::*;

use datastructures::*;
use evmap::{ReadHandle, ReadHandleFactory, WriteHandle};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct Metrics<T: 'static>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    data_read: ReadHandleFactory<String, Arc<Channel<T>>>,
    data_write: Arc<Mutex<WriteHandle<String, Arc<Channel<T>>>>>,
    labels: Arc<Mutex<HashSet<String>>>,
}

pub struct Recorder<T: 'static>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    data_read: ReadHandle<String, Arc<Channel<T>>>,
    data_write: Arc<Mutex<WriteHandle<String, Arc<Channel<T>>>>>,
    labels: Arc<Mutex<HashSet<String>>>,
}

impl<T> Metrics<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    pub fn new() -> Self {
        let (read, write) = evmap::new();
        Self {
            data_read: read.factory(),
            data_write: Arc::new(Mutex::new(write)),
            labels: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn recorder(&self) -> Recorder<T> {
        Recorder {
            data_read: self.data_read.handle(),
            data_write: self.data_write.clone(),
            labels: self.labels.clone(),
        }
    }
}

impl<T> Recorder<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating + From<u8>,
    u64: From<<T as AtomicPrimitive>::Primitive>,
{
    pub fn record(
        &self,
        channel: String,
        measurement: Measurement<<T as AtomicPrimitive>::Primitive>,
    ) {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].record(measurement));
    }

    pub fn counter(&self, channel: String) -> u64 {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].counter())
            .unwrap_or(0)
    }

    pub fn percentile(&self, channel: String, percentile: f64) -> Option<u64> {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].percentile(percentile))
            .unwrap_or(None)
    }

    pub fn add_channel(&self, name: String, source: Source, histogram: Option<Histogram<T>>) {
        debug!("add channel: {} source: {:?}", name, source);
        let channel = Channel::new(name.clone(), source, histogram);
        if self
            .data_read
            .get_and(&name, |channel| channel.len())
            .unwrap_or(0)
            == 0
        {
            let mut write = self.data_write.lock().unwrap();
            write.insert(name.clone(), Arc::new(channel));
            write.refresh();
            let mut labels = self.labels.lock().unwrap();
            labels.insert(name);
        }
    }

    pub fn delete_channel(&self, name: String) {
        debug!("delete channel: {}", name);
        let mut write = self.data_write.lock().unwrap();
        write.empty(name.clone());
        write.refresh();
        let mut labels = self.labels.lock().unwrap();
        labels.remove(&name);
    }

    pub fn readings(&self) -> Vec<Reading> {
        let mut result = Vec::new();
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            let readings = self
                .data_read
                .get_and(label, |channel| (*channel)[0].readings());
            if let Some(readings) = readings {
                result.extend(readings);
            }
        }
        result
    }

    pub fn hash_map(&self) -> HashMap<String, HashMap<Output, u64>> {
        let mut result = HashMap::new();
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            let readings = self
                .data_read
                .get_and(label, |channel| (*channel)[0].hash_map());
            if let Some(readings) = readings {
                result.insert(label.to_owned(), readings);
            }
        }
        result
    }

    #[cfg(feature = "waterfall")]
    pub fn save_files(&self) {
        unsafe {
            for label in &*self.labels.get() {
                let readings = self
                    .data_read
                    .get_and(label, |channel| (*channel)[0].save_files());
            }
        }
    }

    pub fn add_output(&self, name: String, output: Output) {
        self.data_read
            .get_and(&name, |channel| (*channel)[0].add_output(output));
    }

    pub fn delete_output(&self, name: String, output: Output) {
        self.data_read
            .get_and(&name, |channel| (*channel)[0].delete_output(output));
    }

    pub fn latch(&self) {
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            self.data_read
                .get_and(label, |channel| (*channel)[0].latch());
        }
    }

    pub fn zero(&self) {
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            self.data_read
                .get_and(label, |channel| (*channel)[0].zero());
        }
    }

    pub fn clear(&self) {
        let mut labels = self.labels.lock().unwrap();
        let mut write = self.data_write.lock().unwrap();
        labels.clear();
        write.purge();
        write.refresh();
    }

    pub fn shrink_to_fit(&self) {
        let mut write = self.data_write.lock().unwrap();
        write.fit_all();
        write.refresh();
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
