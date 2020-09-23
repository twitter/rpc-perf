// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod http;
mod snapshot;
mod stat;

use crate::Config;
use crate::SECOND;

pub use http::Http;
use rustcommon_heatmap::AtomicHeatmap;
use rustcommon_metrics::*;
use rustcommon_waterfall::{Palette, WaterfallBuilder};
pub use snapshot::MetricsSnapshot;
pub use stat::Stat;
use strum::IntoEnumIterator;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

pub struct StandardOut {
    previous: HashMap<Stat, u64>,
    metrics: Metrics,
    interval: Duration,
}

impl StandardOut {
    pub fn new(metrics: Metrics, interval: Duration) -> Self {
        Self {
            previous: HashMap::new(),
            metrics,
            interval,
        }
    }

    pub fn print(&mut self) {
        let mut current = HashMap::new();
        for stat in [
            Stat::Window,
            Stat::ConnectionsTotal,
            Stat::ConnectionsOpened,
            Stat::ConnectionsError,
            Stat::ConnectionsTimeout,
            Stat::ConnectionsClosed,
            Stat::CommandsGet,
            Stat::CommandsSet,
            Stat::RequestsDequeued,
            Stat::RequestsEnqueued,
            Stat::RequestsTimeout,
            Stat::ResponsesOk,
            Stat::ResponsesError,
            Stat::ResponsesHit,
            Stat::ResponsesMiss,
            Stat::ResponsesTotal,
        ]
        .iter()
        {
            current.insert(*stat, self.metrics.reading(stat).unwrap_or(0));
        }

        info!("-----");
        info!("Window: {}", current.get(&Stat::Window).unwrap());
        info!(
            "Connections: Attempts: {} Opened: {} Errors: {} Timeouts: {} Open: {}",
            self.delta_count(&Stat::ConnectionsTotal, &current),
            self.delta_count(&Stat::ConnectionsOpened, &current),
            self.delta_count(&Stat::ConnectionsError, &current),
            self.delta_count(&Stat::ConnectionsTimeout, &current),
            self.metrics
                .reading(&Stat::ConnectionsOpened)
                .unwrap_or(0)
                .saturating_sub(self.metrics.reading(&Stat::ConnectionsClosed).unwrap_or(0)),
        );
        info!(
            "Commands: Get: {} Set: {}",
            self.delta_count(&Stat::CommandsGet, &current),
            self.delta_count(&Stat::CommandsSet, &current),
        );
        self.display_percentiles(Stat::KeySize, "Keys", 1, "bytes");
        self.display_percentiles(Stat::ValueSize, "Values", 1, "bytes");
        info!(
            "Requests: Sent: {} Timeout: {} Prepared: {} Queue Depth: {}",
            self.delta_count(&Stat::RequestsDequeued, &current),
            self.delta_count(&Stat::RequestsTimeout, &current),
            self.delta_count(&Stat::RequestsEnqueued, &current),
            self.metrics
                .reading(&Stat::RequestsEnqueued)
                .unwrap_or(0)
                .saturating_sub(self.metrics.reading(&Stat::RequestsDequeued).unwrap_or(0)),
        );
        info!(
            "Responses: Ok: {} Error: {} Hit: {} Miss: {}",
            self.delta_count(&Stat::ResponsesOk, &current),
            self.delta_count(&Stat::ResponsesError, &current),
            self.delta_count(&Stat::ResponsesHit, &current),
            self.delta_count(&Stat::ResponsesMiss, &current),
        );
        info!(
            "Rate: Request: {:.2} rps Response: {:.2} rps Connect: {:.2} cps",
            self.rate(&Stat::RequestsDequeued, &current),
            self.rate(&Stat::ResponsesTotal, &current),
            self.rate(&Stat::ConnectionsTotal, &current),
        );
        info!(
            "Success: Request: {:.2}% Response: {:.2}% Connect: {:.2}%",
            self.delta_percent(&Stat::ResponsesTotal, &Stat::RequestsDequeued, &current,),
            self.delta_percent(&Stat::ResponsesOk, &Stat::ResponsesTotal, &current,),
            self.delta_percent(&Stat::ConnectionsOpened, &Stat::ConnectionsTotal, &current,),
        );
        info!(
            "Hit-rate: {:.2}%",
            self.hitrate(&Stat::ResponsesHit, &Stat::ResponsesMiss, &current)
        );
        self.display_percentiles(Stat::ConnectionsLatency, "Connect Latency", 1000, "us");
        self.display_percentiles(Stat::ResponsesLatency, "Request Latency", 1000, "us");
        self.previous = current;
    }

    fn rate(&self, stat: &Stat, current: &HashMap<Stat, u64>) -> f64 {
        let dv = self.delta_count(stat, current) as f64;
        let dt =
            self.interval.as_secs() as f64 + self.interval.subsec_nanos() as f64 / 1000000000.0;
        dv / dt
    }

    fn delta_count(&self, stat: &Stat, current: &HashMap<Stat, u64>) -> u64 {
        current
            .get(stat)
            .unwrap_or(&0)
            .saturating_sub(*self.previous.get(stat).unwrap_or(&0))
    }

    fn delta_percent(&self, a: &Stat, b: &Stat, current: &HashMap<Stat, u64>) -> f64 {
        let da = self.delta_count(a, current) as f64;
        let db = self.delta_count(b, current) as f64;
        if db == 0.0 {
            100.0
        } else {
            100.0 * da / db
        }
    }

    fn hitrate(&self, a: &Stat, b: &Stat, current: &HashMap<Stat, u64>) -> f64 {
        let da = self.delta_count(a, current) as f64;
        let db = da + self.delta_count(b, current) as f64;
        if db == 0.0 {
            100.0
        } else {
            100.0 * da / db
        }
    }

    fn display_percentiles(&self, stat: Stat, label: &str, divisor: u64, unit: &str) {
        let p25 = self
            .metrics
            .percentile(&stat, 25.0)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p50 = self
            .metrics
            .percentile(&stat, 50.0)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p75 = self
            .metrics
            .percentile(&stat, 75.0)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p90 = self
            .metrics
            .percentile(&stat, 90.0)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p99 = self
            .metrics
            .percentile(&stat, 99.0)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p999 = self
            .metrics
            .percentile(&stat, 99.9)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        let p9999 = self
            .metrics
            .percentile(&stat, 99.99)
            .map(|v| format!("{}", v / divisor))
            .unwrap_or_else(|_| "none".to_string());
        info!(
            "{} ({}): p25: {} p50: {} p75: {} p90: {} p99: {} p999: {} p9999: {}",
            label, unit, p25, p50, p75, p90, p99, p999, p9999
        );
    }
}

#[derive(Clone)]
pub struct Metrics {
    inner: Arc<rustcommon_metrics::Metrics<AtomicU64, AtomicU32>>,
    heatmap: Arc<Option<Arc<AtomicHeatmap<u64, AtomicU32>>>>,
    config: Arc<Config>,
}

impl Metrics {
    pub fn inner(&self) -> Arc<rustcommon_metrics::Metrics<AtomicU64, AtomicU32>> {
        self.inner.clone()
    }

    pub fn reading(&self, stat: &Stat) -> Result<u64, MetricsError> {
        self.inner.reading(stat)
    }

    pub fn percentile(&self, stat: &Stat, percentile: f64) -> Result<u64, MetricsError> {
        self.inner.percentile(stat, percentile)
    }

    pub fn new(config: Arc<Config>) -> Self {
        let heatmap = if config.waterfall().is_some() {
            if let Some(windows) = config.windows() {
                Some(Arc::new(AtomicHeatmap::new(
                    SECOND as u64,
                    3,
                    windows as usize * config.interval(),
                    Duration::new(1, 0),
                )))
            } else {
                warn!("Unable to initialize waterfall output without fixed duration");
                None
            }
        } else {
            None
        };
        let metrics = Self {
            inner: Arc::new(rustcommon_metrics::Metrics::new()),
            heatmap: Arc::new(heatmap),
            config,
        };
        metrics.register();
        metrics
    }

    pub fn register(&self) {
        for stat in Stat::iter() {
            self.inner.register(&stat);
            match stat {
                Stat::ConnectionsLatency | Stat::ResponsesLatency | Stat::KeySize | Stat::ValueSize => {
                    // use heatmaps with 10 slices, each at 1/10th the interval
                    self.inner.add_summary(
                        &stat,
                        Summary::heatmap(
                            1_000_000_000,
                            3,
                            10,
                            Duration::from_millis(self.config.interval() as u64 * 100),
                        ),
                    );
                }
                _ => {}
            }
            self.inner.add_output(&stat, Output::Reading);
            for percentile in &[50.0, 75.0, 90.0, 99.0, 99.9, 99.99] {
                self.inner
                    .add_output(&stat, Output::Percentile(*percentile))
            }
        }
    }

    pub fn increment(&self, statistic: &dyn Statistic<AtomicU64, AtomicU32>) {
        let _ = self.inner.increment_counter(statistic, 1);
    }

    pub fn time_interval(
        &self,
        statistic: &dyn Statistic<AtomicU64, AtomicU32>,
        start: Instant,
        stop: Instant,
    ) {
        let duration = stop - start;
        let value = duration.as_secs() * SECOND as u64 + duration.subsec_nanos() as u64;
        let _ = self.inner.record_bucket(statistic, start, value, 1);
    }

    pub fn distribution(&self, statistic: &dyn Statistic<AtomicU64, AtomicU32>, value: u64) {
        let _ = self
            .inner
            .record_bucket(statistic, Instant::now(), value, 1);
    }

    pub fn zero(&self) {
        self.inner.clear();
        self.register();
    }

    pub fn heatmap_increment(&self, start: Instant, stop: Instant) {
        let latency = stop - start;
        let latency = latency.as_secs() * SECOND as u64 + latency.subsec_nanos() as u64;
        if let Some(ref heatmap) = *self.heatmap {
            heatmap.increment(start, latency, 1);
        }
    }

    pub fn save_waterfall(&self, file: String) {
        if let Some(ref heatmap) = *self.heatmap {
            WaterfallBuilder::new(&file)
                .palette(Palette::Ironbow)
                .label(100, "100ns")
                .label(200, "200ns")
                .label(400, "400ns")
                .label(1_000, "1us")
                .label(2_000, "2us")
                .label(4_000, "4us")
                .label(10_000, "10us")
                .label(20_000, "20us")
                .label(40_000, "40us")
                .label(100_000, "100us")
                .label(200_000, "200us")
                .label(400_000, "400us")
                .label(1_000_000, "1ms")
                .label(2_000_000, "2ms")
                .label(4_000_000, "4ms")
                .label(10_000_000, "10ms")
                .label(20_000_000, "20ms")
                .label(40_000_000, "40ms")
                .label(100_000_000, "100ms")
                .label(200_000_000, "200ms")
                .label(400_000_000, "400ms")
                .build(&heatmap.load());
        }
    }
}
