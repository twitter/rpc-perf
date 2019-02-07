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

pub mod http;

use crate::client::SECOND;
use crate::config::Config;
pub use crate::stats::http::Http;
use datastructures::Heatmap;

use metrics::*;

use std::collections::HashMap;

use logger::*;

pub fn register_stats(recorder: &Simple) {
    recorder.add_counter_channel(Stat::CommandsGet);
    recorder.add_counter_channel(Stat::CommandsSet);
    recorder.add_distribution_channel(Stat::KeySize, 0, 60_000_000_000, 3);
    recorder.add_distribution_channel(Stat::ValueSize, 0, 60_000_000_000, 3);
    recorder.add_counter_channel(Stat::Window);
    recorder.add_counter_channel(Stat::RequestsEnqueued);
    recorder.add_counter_channel(Stat::RequestsDequeued);
    recorder.add_counter_channel(Stat::RequestsError);
    recorder.add_counter_channel(Stat::RequestsTimeout);
    recorder.add_counter_channel(Stat::ConnectionsTotal);
    recorder.add_histogram_channel(Stat::ConnectionsOpened, 0, 60_000_000_000, 3);
    recorder.add_counter_channel(Stat::ConnectionsClosed);
    recorder.add_counter_channel(Stat::ConnectionsError);
    recorder.add_counter_channel(Stat::ConnectionsClientClosed);
    recorder.add_counter_channel(Stat::ConnectionsServerClosed);
    recorder.add_counter_channel(Stat::ConnectionsTimeout);
    recorder.add_histogram_channel(Stat::ResponsesTotal, 0, 60_000_000_000, 3);
    recorder.add_counter_channel(Stat::ResponsesOk);
    recorder.add_counter_channel(Stat::ResponsesError);
    recorder.add_counter_channel(Stat::ResponsesHit);
    recorder.add_counter_channel(Stat::ResponsesMiss);
}

pub struct StandardOut<'a> {
    previous: HashMap<String, HashMap<Output, usize>>,
    recorder: &'a Simple,
    interval: usize,
}

impl<'a> StandardOut<'a> {
    pub fn new(recorder: &'a Simple, interval: usize) -> Self {
        Self {
            previous: recorder.hash_map(),
            recorder,
            interval,
        }
    }

    fn display_percentiles(&self, stat: Stat, label: &str, divisor: usize, unit: &str) {
        let p25 = self
            .recorder
            .percentile(stat, 0.25)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p50 = self
            .recorder
            .percentile(stat, 0.50)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p75 = self
            .recorder
            .percentile(stat, 0.75)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p90 = self
            .recorder
            .percentile(stat, 0.90)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p99 = self
            .recorder
            .percentile(stat, 0.99)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p999 = self
            .recorder
            .percentile(stat, 0.999)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p9999 = self
            .recorder
            .percentile(stat, 0.9999)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        info!(
            "{} ({}): p25: {} p50: {} p75: {} p90: {} p99: {} p999: {} p9999: {}",
            label, unit, p25, p50, p75, p90, p99, p999, p9999
        );
    }

    pub fn print(&mut self) {
        let current = self.recorder.hash_map();
        let window = self.recorder.counter(Stat::Window);
        info!("-----");
        info!("Window: {}", window);

        // connections
        info!(
            "Connections: Attempts: {} Opened: {} Errors: {} Timeouts: {} Open: {}",
            delta_count(&self.previous, &current, Stat::ConnectionsTotal).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ConnectionsOpened).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ConnectionsError).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ConnectionsTimeout).unwrap_or(0),
            self.recorder
                .counter(Stat::ConnectionsOpened)
                .saturating_sub(self.recorder.counter(Stat::ConnectionsClosed)),
        );

        info!(
            "Commands: Get: {} Set: {}",
            delta_count(&self.previous, &current, Stat::CommandsGet).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::CommandsSet).unwrap_or(0),
        );

        self.display_percentiles(Stat::KeySize, "Keys", 1, "bytes");
        self.display_percentiles(Stat::ValueSize, "Values", 1, "bytes");

        info!(
            "Requests: Sent: {} Timeout: {} Prepared: {} Queue Depth: {}",
            delta_count(&self.previous, &current, Stat::RequestsDequeued).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::RequestsTimeout).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::RequestsEnqueued).unwrap_or(0),
            self.recorder.counter(Stat::RequestsEnqueued)
                - self.recorder.counter(Stat::RequestsDequeued),
        );

        info!(
            "Responses: Ok: {} Error: {} Hit: {} Miss: {}",
            delta_count(&self.previous, &current, Stat::ResponsesOk).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ResponsesError).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ResponsesHit).unwrap_or(0),
            delta_count(&self.previous, &current, Stat::ResponsesMiss).unwrap_or(0),
        );

        info!(
            "Rate: Request: {:.2} rps Response: {:.2} rps Connect: {:.2} cps",
            delta_count(&self.previous, &current, Stat::RequestsDequeued).unwrap_or(0) as f64
                / self.interval as f64,
            delta_count(&self.previous, &current, Stat::ResponsesTotal).unwrap_or(0) as f64
                / self.interval as f64,
            delta_count(&self.previous, &current, Stat::ConnectionsTotal).unwrap_or(0) as f64
                / self.interval as f64,
        );

        info!(
            "Success: Request: {:.2}% Response: {:.2}% Connect: {:.2}%",
            delta_percent(
                &self.previous,
                &current,
                Stat::ResponsesTotal,
                Stat::RequestsDequeued
            )
            .unwrap_or(100.0),
            delta_percent(
                &self.previous,
                &current,
                Stat::ResponsesOk,
                Stat::ResponsesTotal
            )
            .unwrap_or(100.0),
            delta_percent(
                &self.previous,
                &current,
                Stat::ConnectionsOpened,
                Stat::ConnectionsTotal
            )
            .unwrap_or(100.0),
        );

        let hit = delta_count(&self.previous, &current, Stat::ResponsesHit).unwrap_or(0);
        let miss = delta_count(&self.previous, &current, Stat::ResponsesMiss).unwrap_or(0);
        let hitrate = 100.0 * hit as f64 / (hit + miss) as f64;

        info!("Hit-rate: {:.2}%", hitrate);

        self.display_percentiles(Stat::ConnectionsOpened, "Connect Latency", 1000, "us");
        self.display_percentiles(Stat::ResponsesTotal, "Request Latency", 1000, "us");
        self.previous = current;
    }
}

#[derive(Copy, Clone)]
pub enum Stat {
    Window,
    RequestsEnqueued,
    RequestsDequeued,
    RequestsError,
    RequestsTimeout,
    ConnectionsTotal,
    ConnectionsOpened,
    ConnectionsClosed,
    ConnectionsError,
    ConnectionsClientClosed,
    ConnectionsServerClosed,
    ConnectionsTimeout,
    ResponsesTotal,
    ResponsesOk,
    ResponsesError,
    ResponsesHit,
    ResponsesMiss,
    CommandsGet,
    CommandsSet,
    KeySize,
    ValueSize,
}

impl ToString for Stat {
    fn to_string(&self) -> String {
        let label = match self {
            Stat::CommandsGet => "commands/get",
            Stat::CommandsSet => "commands/set",
            Stat::KeySize => "keys/size",
            Stat::ValueSize => "values/size",
            Stat::Window => "window",
            Stat::RequestsEnqueued => "requests/enqueued",
            Stat::RequestsDequeued => "requests/dequeued",
            Stat::RequestsError => "requests/error",
            Stat::RequestsTimeout => "requests/timeout",
            Stat::ConnectionsTotal => "connections/total",
            Stat::ConnectionsOpened => "connections/opened",
            Stat::ConnectionsClosed => "connections/closed/total",
            Stat::ConnectionsError => "connections/error",
            Stat::ConnectionsClientClosed => "connections/closed/client",
            Stat::ConnectionsServerClosed => "connections/closed/server",
            Stat::ConnectionsTimeout => "connections/timeout",
            Stat::ResponsesTotal => "responses/total",
            Stat::ResponsesOk => "responses/ok",
            Stat::ResponsesError => "responses/error",
            Stat::ResponsesHit => "responses/hit",
            Stat::ResponsesMiss => "responses/miss",
        };
        label.to_string()
    }
}

fn delta_count<T: ToString>(
    a: &HashMap<String, HashMap<Output, usize>>,
    b: &HashMap<String, HashMap<Output, usize>>,
    label: T,
) -> Option<usize> {
    let output = Output::Counter;
    let label = label.to_string();
    if let Some(a_outputs) = a.get(&label) {
        let a_value = a_outputs.get(&output).unwrap_or(&0);
        if let Some(b_outputs) = b.get(&label) {
            let b_value = b_outputs.get(&output).unwrap_or(&0);

            Some(b_value - a_value)
        } else {
            None
        }
    } else {
        None
    }
}

fn delta_percent<T: ToString>(
    a: &HashMap<String, HashMap<Output, usize>>,
    b: &HashMap<String, HashMap<Output, usize>>,
    label_a: T,
    label_b: T,
) -> Option<f64> {
    let delta_a = delta_count(a, b, label_a);
    let delta_b = delta_count(a, b, label_b);

    if let Some(a) = delta_a {
        if let Some(b) = delta_b {
            if b == 0 {
                Some(100.0)
            } else {
                Some(100.0 * a as f64 / b as f64)
            }
        } else {
            Some(100.0)
        }
    } else {
        Some(0.0)
    }
}

#[derive(Clone)]
pub struct Simple {
    inner: Recorder,
    heatmap: Option<Heatmap>,
}

impl Simple {
    pub fn new(config: &Config) -> Self {
        let heatmap = if config.waterfall().is_some() {
            if let Some(windows) = config.windows() {
                Some(
                    datastructures::HeatmapBuilder::new(
                        0,
                        SECOND,
                        2,
                        SECOND,
                        windows * config.interval() * SECOND,
                    )
                    .build(),
                )
            } else {
                warn!("Unable to initialize waterfall output without fixed duration");
                None
            }
        } else {
            None
        };
        Self {
            inner: Recorder::new(),
            heatmap,
        }
    }

    pub fn add_counter_channel<T: ToString>(&self, label: T) {
        self.inner
            .add_channel(label.to_string(), Source::Counter, None);
        self.inner.add_output(label.to_string(), Output::Counter);
    }

    pub fn add_histogram_channel<T: ToString>(
        &self,
        label: T,
        min: usize,
        max: usize,
        precision: usize,
    ) {
        let histogram_config = HistogramConfig::new(min, max, precision, None);
        self.inner.add_channel(
            label.to_string(),
            Source::TimeInterval,
            Some(histogram_config),
        );
        self.inner.add_output(label.to_string(), Output::Counter);
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p50));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p75));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p90));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p99));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p999));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p9999));
    }

    pub fn add_distribution_channel<T: ToString>(
        &self,
        label: T,
        min: usize,
        max: usize,
        precision: usize,
    ) {
        let histogram_config = HistogramConfig::new(min, max, precision, None);
        self.inner.add_channel(
            label.to_string(),
            Source::Distribution,
            Some(histogram_config),
        );
        self.inner.add_output(label.to_string(), Output::Counter);
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p50));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p75));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p90));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p99));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p999));
        self.inner
            .add_output(label.to_string(), Output::Percentile(Percentile::p9999));
    }

    pub fn counter<T: ToString>(&self, label: T) -> usize {
        self.inner.counter(label.to_string())
    }

    pub fn increment<T: ToString>(&self, label: T) {
        self.inner.record(
            label.to_string(),
            Measurement::Increment {
                time: time::precise_time_ns() as usize,
                value: 1,
            },
        )
    }

    pub fn time_interval<T: ToString>(&self, label: T, start: usize, stop: usize) {
        self.inner
            .record(label.to_string(), Measurement::TimeInterval { start, stop });
    }

    pub fn heatmap_increment(&self, start: usize, stop: usize) {
        if let Some(ref heatmap) = self.heatmap {
            heatmap.incr(start, stop - start, 1);
        }
    }

    pub fn distribution<T: ToString>(&self, label: T, value: usize) {
        self.inner.record(
            label.to_string(),
            Measurement::Distribution {
                time: time::precise_time_ns() as usize,
                value,
                count: 1,
            },
        );
    }

    pub fn percentile<T: ToString>(&self, label: T, percentile: f64) -> Option<usize> {
        self.inner.percentile(label.to_string(), percentile)
    }

    pub fn latch(&self) {
        self.inner.latch();
    }

    pub fn hash_map(&self) -> HashMap<String, HashMap<Output, usize>> {
        self.inner.hash_map()
    }

    pub fn clear(&self) {
        self.inner.clear();
    }

    pub fn readings(&self) -> Vec<Reading> {
        self.inner.readings()
    }

    pub fn save_waterfall(&self, file: String) {
        if let Some(ref heatmap) = self.heatmap {
            let mut labels = HashMap::new();
            labels.insert(100, "100ns".to_string());
            labels.insert(200, "200ns".to_string());
            labels.insert(400, "400ns".to_string());
            labels.insert(1_000, "1us".to_string());
            labels.insert(2_000, "2us".to_string());
            labels.insert(4_000, "4us".to_string());
            labels.insert(10_000, "10us".to_string());
            labels.insert(20_000, "20us".to_string());
            labels.insert(40_000, "40us".to_string());
            labels.insert(100_000, "100us".to_string());
            labels.insert(200_000, "200us".to_string());
            labels.insert(400_000, "400us".to_string());
            labels.insert(1_000_000, "1ms".to_string());
            labels.insert(2_000_000, "2ms".to_string());
            labels.insert(4_000_000, "4ms".to_string());
            labels.insert(10_000_000, "10ms".to_string());
            labels.insert(20_000_000, "20ms".to_string());
            labels.insert(40_000_000, "40ms".to_string());
            labels.insert(100_000_000, "100ms".to_string());
            labels.insert(200_000_000, "200ms".to_string());
            labels.insert(400_000_000, "400ms".to_string());
            waterfall::save_waterfall(&heatmap, &file, labels, 60 * SECOND);
        }
    }
}
