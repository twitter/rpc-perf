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

use crate::*;
use atomics::AtomicCounter;
use datastructures::UnsignedCounterPrimitive;
use evmap::{ReadHandle, WriteHandle};
use std::sync::Mutex;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct Recorder<T: 'static>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive + From<u8>,
    u64: From<T>,
{
    data_read: ReadHandle<String, Arc<Channel<T>>>,
    data_write: Arc<Mutex<WriteHandle<String, Arc<Channel<T>>>>>,
    labels: Arc<Mutex<HashSet<String>>>,
}

impl<T> Recorder<T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive + From<u8>,
    u64: From<T>,
{
    pub fn new() -> Self {
        let (read, write) = evmap::new();
        Self {
            data_read: read,
            data_write: Arc::new(Mutex::new(write)),
            labels: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn record(&self, channel: String, measurement: Measurement<T>) {
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
        let labels = self.labels.lock().unwrap();
        for label in &*labels {
            self.delete_channel(label.to_string());
        }
    }
}

impl<T> Default for Recorder<T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive + From<u8>,
    u64: From<T>,
{
    fn default() -> Self {
        Self::new()
    }
}
