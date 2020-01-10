// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub mod http;
mod stat;

pub use stat::Stat;

pub use crate::stats::http::Http;

use crate::client::SECOND;
use crate::config::Config;

use datastructures::Heatmap;
use logger::*;
use metrics::{self, Output, Percentile, Reading, Source, Statistic, Summary};

use std::collections::HashMap;
use std::sync::Arc;

pub fn register_stats(metrics: &Metrics) {
    for statistic in &[
        Stat::CommandsDelete,
        Stat::CommandsGet,
        Stat::CommandsRange,
        Stat::CommandsSet,
        Stat::KeySize,
        Stat::ValueSize,
        Stat::Window,
        Stat::RequestsEnqueued,
        Stat::RequestsDequeued,
        Stat::ConnectionsTotal,
        Stat::ConnectionsOpened,
        Stat::ConnectionsClosed,
        Stat::ConnectionsError,
        Stat::ConnectionsClientClosed,
        Stat::ConnectionsServerClosed,
        Stat::ConnectionsTimeout,
        Stat::ResponsesTotal,
        Stat::ResponsesOk,
        Stat::ResponsesError,
        Stat::ResponsesHit,
        Stat::ResponsesMiss,
    ] {
        metrics.register(statistic);
    }
}

pub struct StandardOut {
    previous: HashMap<String, HashMap<metrics::Output, u64>>,
    metrics: Metrics,
    interval: u64,
}

impl StandardOut {
    pub fn new(metrics: Metrics, interval: u64) -> Self {
        Self {
            previous: metrics.hash_map(),
            metrics,
            interval,
        }
    }

    fn display_percentiles(&self, stat: Stat, label: &str, divisor: u64, unit: &str) {
        let p25 = self
            .metrics
            .percentile(&stat, 0.25)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p50 = self
            .metrics
            .percentile(&stat, 0.50)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p75 = self
            .metrics
            .percentile(&stat, 0.75)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p90 = self
            .metrics
            .percentile(&stat, 0.90)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p99 = self
            .metrics
            .percentile(&stat, 0.99)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p999 = self
            .metrics
            .percentile(&stat, 0.999)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        let p9999 = self
            .metrics
            .percentile(&stat, 0.9999)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|| "none".to_string());
        info!(
            "{} ({}): p25: {} p50: {} p75: {} p90: {} p99: {} p999: {} p9999: {}",
            label, unit, p25, p50, p75, p90, p99, p999, p9999
        );
    }

    pub fn print(&mut self) {
        let current = self.metrics.hash_map();
        let window = self.metrics.counter(&Stat::Window);
        info!("-----");
        info!("Window: {}", window);

        // connections
        info!(
            "Connections: Attempts: {} Opened: {} Errors: {} Timeouts: {} Open: {}",
            delta_count(&self.previous, &current, &Stat::ConnectionsTotal).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ConnectionsOpened).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ConnectionsError).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ConnectionsTimeout).unwrap_or(0),
            self.metrics
                .counter(&Stat::ConnectionsOpened)
                .saturating_sub(self.metrics.counter(&Stat::ConnectionsClosed)),
        );

        info!(
            "Commands: Get: {} Set: {}",
            delta_count(&self.previous, &current, &Stat::CommandsGet).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::CommandsSet).unwrap_or(0),
        );

        self.display_percentiles(Stat::KeySize, "Keys", 1, "bytes");
        self.display_percentiles(Stat::ValueSize, "Values", 1, "bytes");

        info!(
            "Requests: Sent: {} Timeout: {} Prepared: {} Queue Depth: {}",
            delta_count(&self.previous, &current, &Stat::RequestsDequeued).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::RequestsTimeout).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::RequestsEnqueued).unwrap_or(0),
            self.metrics.counter(&Stat::RequestsEnqueued)
                - self.metrics.counter(&Stat::RequestsDequeued),
        );

        info!(
            "Responses: Ok: {} Error: {} Hit: {} Miss: {}",
            delta_count(&self.previous, &current, &Stat::ResponsesOk).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ResponsesError).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ResponsesHit).unwrap_or(0),
            delta_count(&self.previous, &current, &Stat::ResponsesMiss).unwrap_or(0),
        );

        info!(
            "Rate: Request: {:.2} rps Response: {:.2} rps Connect: {:.2} cps",
            delta_count(&self.previous, &current, &Stat::RequestsDequeued).unwrap_or(0) as f64
                / self.interval as f64,
            delta_count(&self.previous, &current, &Stat::ResponsesTotal).unwrap_or(0) as f64
                / self.interval as f64,
            delta_count(&self.previous, &current, &Stat::ConnectionsTotal).unwrap_or(0) as f64
                / self.interval as f64,
        );

        info!(
            "Success: Request: {:.2}% Response: {:.2}% Connect: {:.2}%",
            delta_percent(
                &self.previous,
                &current,
                &Stat::ResponsesTotal,
                &Stat::RequestsDequeued
            )
            .unwrap_or(100.0),
            delta_percent(
                &self.previous,
                &current,
                &Stat::ResponsesOk,
                &Stat::ResponsesTotal
            )
            .unwrap_or(100.0),
            delta_percent(
                &self.previous,
                &current,
                &Stat::ConnectionsOpened,
                &Stat::ConnectionsTotal
            )
            .unwrap_or(100.0),
        );

        let hit = delta_count(&self.previous, &current, &Stat::ResponsesHit).unwrap_or(0);
        let miss = delta_count(&self.previous, &current, &Stat::ResponsesMiss).unwrap_or(0);
        let hitrate = 100.0 * hit as f64 / (hit + miss) as f64;

        info!("Hit-rate: {:.2}%", hitrate);

        self.display_percentiles(Stat::ConnectionsOpened, "Connect Latency", 1000, "us");
        self.display_percentiles(Stat::ResponsesTotal, "Request Latency", 1000, "us");
        self.previous = current;
    }
}

fn delta_count(
    a: &HashMap<String, HashMap<metrics::Output, u64>>,
    b: &HashMap<String, HashMap<metrics::Output, u64>>,
    label: &dyn Statistic,
) -> Option<u64> {
    let output = metrics::Output::Reading;
    let label = label.name();
    if let Some(a_outputs) = a.get(label) {
        let a_value = a_outputs.get(&output).unwrap_or(&0);
        if let Some(b_outputs) = b.get(label) {
            let b_value = b_outputs.get(&output).unwrap_or(&0);

            Some(b_value - a_value)
        } else {
            None
        }
    } else {
        None
    }
}

fn delta_percent(
    a: &HashMap<String, HashMap<metrics::Output, u64>>,
    b: &HashMap<String, HashMap<metrics::Output, u64>>,
    label_a: &dyn Statistic,
    label_b: &dyn Statistic,
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
pub struct Metrics {
    inner: Arc<metrics::Metrics<metrics::AtomicU64>>,
    heatmap: Arc<Option<Arc<Heatmap<metrics::AtomicU64>>>>,
}

impl Metrics {
    pub fn new(config: &Config) -> Self {
        let heatmap = if config.waterfall().is_some() {
            if let Some(windows) = config.windows() {
                Some(Arc::new(Heatmap::new(
                    SECOND as u64,
                    3,
                    SECOND as u64,
                    (windows * config.interval() * SECOND) as u64,
                )))
            } else {
                warn!("Unable to initialize waterfall output without fixed duration");
                None
            }
        } else {
            None
        };
        Self {
            inner: Arc::new(metrics::Metrics::new()),
            heatmap: Arc::new(heatmap),
        }
    }

    pub fn register(&self, statistic: &dyn Statistic) {
        let summary = match statistic.source() {
            Source::TimeInterval => Some(Summary::histogram(60_000_000_000, 3, None)),
            Source::Distribution => Some(Summary::histogram(1_000_000_000, 3, None)),
            _ => None,
        };
        self.inner.register(statistic, summary);
        self.inner
            .register_output(statistic, metrics::Output::Reading);
        if summary.is_some() {
            for percentile in &[
                Percentile::p50,
                Percentile::p75,
                Percentile::p90,
                Percentile::p99,
                Percentile::p999,
                Percentile::p9999,
            ] {
                self.inner
                    .register_output(statistic, metrics::Output::Percentile(*percentile));
            }
        }
    }

    pub fn counter(&self, statistic: &dyn Statistic) -> u64 {
        self.inner.reading(statistic).unwrap_or(0)
    }

    pub fn increment(&self, statistic: &dyn Statistic) {
        self.inner
            .record_increment(statistic, time::precise_time_ns(), 1)
    }

    pub fn time_interval(&self, statistic: &dyn Statistic, start: u64, stop: u64) {
        self.inner.record_time_interval(statistic, start, stop);
    }

    pub fn heatmap_increment(&self, start: u64, stop: u64) {
        if let Some(ref heatmap) = *self.heatmap {
            heatmap.increment(start, stop - start, 1);
        }
    }

    pub fn distribution(&self, statistic: &dyn Statistic, value: u64) {
        self.inner
            .record_distribution(statistic, time::precise_time_ns(), value, 1);
    }

    pub fn percentile(&self, statistic: &dyn Statistic, percentile: f64) -> Option<u64> {
        self.inner.percentile(statistic, percentile)
    }

    pub fn latch(&self) {
        self.inner.latch();
    }

    pub fn hash_map(&self) -> HashMap<String, HashMap<Output, u64>> {
        self.inner.hash_map()
    }

    pub fn zero(&self) {
        self.inner.zero();
    }

    pub fn readings(&self) -> Vec<Reading> {
        self.inner.readings()
    }

    pub fn save_waterfall(&self, file: String) {
        if let Some(ref heatmap) = *self.heatmap {
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
            waterfall::save_waterfall(&heatmap, &file, labels, 60 * SECOND as u64);
        }
    }
}
