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
use datastructures::HistogramConfig;
use datastructures::RwWrapper;
use std::collections::HashSet;
use std::sync::Arc;

use datastructures::Wrapper;

use evmap::{ReadHandle, WriteHandle};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Recorder {
    data_read: ReadHandle<String, Arc<Channel>>,
    data_write: Wrapper<WriteHandle<String, Arc<Channel>>>,
    labels: RwWrapper<HashSet<String>>,
}

impl Recorder {
    pub fn new() -> Self {
        let (read, write) = evmap::new();
        Self {
            data_read: read,
            data_write: Wrapper::new(write),
            labels: RwWrapper::new(HashSet::new()),
        }
    }

    pub fn record(&self, channel: String, measurement: Measurement) {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].record(measurement));
    }

    pub fn counter(&self, channel: String) -> usize {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].counter())
            .unwrap_or(0)
    }

    pub fn percentile(&self, channel: String, percentile: f64) -> Option<usize> {
        self.data_read
            .get_and(&channel, |channel| (*channel)[0].percentile(percentile))
            .unwrap_or(None)
    }

    pub fn add_channel(
        &self,
        name: String,
        source: Source,
        histogram_config: Option<HistogramConfig>,
    ) {
        debug!("add channel: {} source: {:?}", name, source);
        let channel = Channel::new(name.clone(), source, histogram_config);
        if self
            .data_read
            .get_and(&name, |channel| channel.len())
            .unwrap_or(0)
            == 0
        {
            unsafe {
                (*self.data_write.get()).insert(name.clone(), Arc::new(channel));
                (*self.data_write.get()).refresh();
                (*self.labels.lock()).insert(name);
            }
        }
    }

    pub fn delete_channel(&self, name: String) {
        debug!("delete channel: {}", name);
        unsafe {
            (*self.data_write.get()).empty(name.clone());
            (*self.data_write.get()).refresh();
            (*self.labels.lock()).remove(&name);
        }
    }

    pub fn readings(&self) -> Vec<Reading> {
        let mut result = Vec::new();
        unsafe {
            for label in &*self.labels.get() {
                let readings = self
                    .data_read
                    .get_and(label, |channel| (*channel)[0].readings());
                if let Some(readings) = readings {
                    result.extend(readings);
                }
            }
        }
        result
    }

    pub fn hash_map(&self) -> HashMap<String, HashMap<Output, usize>> {
        let mut result = HashMap::new();
        unsafe {
            for label in &*self.labels.get() {
                let readings = self
                    .data_read
                    .get_and(label, |channel| (*channel)[0].hash_map());
                if let Some(readings) = readings {
                    result.insert(label.to_owned(), readings);
                }
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
        unsafe {
            for label in &*self.labels.get() {
                self.data_read
                    .get_and(label, |channel| (*channel)[0].latch());
            }
        }
    }

    pub fn clear(&self) {
        unsafe {
            for label in &*self.labels.get() {
                self.data_read
                    .get_and(label, |channel| (*channel)[0].clear());
            }
        }
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}
