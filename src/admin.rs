// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::metrics::Metric;
use crate::Arc;
use crate::Config;
use rustcommon_fastmetrics::{Metric as MetricTrait, Source};
use rustcommon_heatmap::AtomicHeatmap;
use rustcommon_heatmap::AtomicU64;
use rustcommon_ratelimiter::Ratelimiter;
use std::collections::HashMap;
use std::time::Instant;

use strum::IntoEnumIterator;
use tiny_http::{Method, Response, Server};

pub struct Admin {
    config: Arc<Config>,
    snapshot: Snapshot,
    connect_heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>,
    request_heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>,
    request_ratelimit: Option<Arc<Ratelimiter>>,
    server: Option<Server>,
}

impl Admin {
    pub fn new(config: Arc<Config>) -> Self {
        let snapshot = Snapshot::new(None, None);
        let server = if let Some(admin_addr) = config.general().admin() {
            Some(Server::http(admin_addr).unwrap())
        } else {
            None
        };

        Self {
            config,
            snapshot,
            connect_heatmap: None,
            request_heatmap: None,
            request_ratelimit: None,
            server,
        }
    }

    pub fn set_connect_heatmap(&mut self, heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>) {
        self.connect_heatmap = heatmap;
    }

    pub fn set_request_heatmap(&mut self, heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>) {
        self.request_heatmap = heatmap;
    }

    pub fn set_request_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.request_ratelimit = ratelimiter;
    }

    pub fn run(mut self) {
        let mut next = Instant::now() + self.config.general().interval();
        let mut snapshot =
            Snapshot::new(self.connect_heatmap.as_ref(), self.request_heatmap.as_ref());
        loop {
            while Instant::now() < next {
                snapshot =
                    Snapshot::new(self.connect_heatmap.as_ref(), self.request_heatmap.as_ref());
                if let Some(ref server) = self.server {
                    while let Ok(Some(mut request)) = server.try_recv() {
                        let url = request.url();
                        let parts: Vec<&str> = url.split('?').collect();
                        let url = parts[0];
                        match request.method() {
                            Method::Get => match url {
                                "/" => {
                                    debug!("Serving GET on index");
                                    let _ = request.respond(Response::from_string(format!(
                                        "Welcome to {}\nVersion: {}\n",
                                        crate::config::NAME,
                                        crate::config::VERSION,
                                    )));
                                }
                                "/metrics" => {
                                    debug!("Serving Prometheus compatible stats");
                                    let _ = request
                                        .respond(Response::from_string(self.snapshot.prometheus()));
                                }
                                "/metrics.json" | "/vars.json" | "/admin/metrics.json" => {
                                    debug!("Serving machine readable stats");
                                    let _ = request
                                        .respond(Response::from_string(self.snapshot.json()));
                                }
                                "/vars" => {
                                    debug!("Serving human readable stats");
                                    let _ = request
                                        .respond(Response::from_string(self.snapshot.human()));
                                }
                                url => {
                                    debug!("GET on non-existent url: {}", url);
                                    debug!("Serving machine readable stats");
                                    let _ = request
                                        .respond(Response::from_string(self.snapshot.json()));
                                }
                            },
                            Method::Put => match request.url() {
                                "/ratelimit/request" => {
                                    let mut content = String::new();
                                    request.as_reader().read_to_string(&mut content).unwrap();
                                    if let Ok(rate) = content.parse() {
                                        if let Some(ref ratelimiter) = self.request_ratelimit {
                                            ratelimiter.set_rate(rate);
                                            let _ = request.respond(Response::empty(200));
                                        } else {
                                            let _ = request.respond(Response::empty(400));
                                        }
                                    } else {
                                        let _ = request.respond(Response::empty(400));
                                    }
                                }
                                url => {
                                    debug!("PUT on non-existent url: {}", url);
                                    let _ = request.respond(Response::empty(404));
                                }
                            },
                            method => {
                                debug!("unsupported request method: {}", method);
                                let _ = request.respond(Response::empty(404));
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            next += self.config.general().interval();

            let window = get_counter!(&Metric::Window).unwrap_or(0);

            info!("-----");
            info!("Window: {}", window);
            info!(
                "Connections: Attempts: {} Opened: {} Errors: {} Timeouts: {} Open: {}",
                snapshot.delta_count(&self.snapshot, Metric::Connect),
                snapshot.delta_count(&self.snapshot, Metric::Session),
                snapshot.delta_count(&self.snapshot, Metric::ConnectEx),
                snapshot.delta_count(&self.snapshot, Metric::ConnectTimeout),
                get_gauge!(&Metric::Open).unwrap_or(0),
            );

            let request_rate = snapshot.rate(&self.snapshot, Metric::Request);
            let response_rate = snapshot.rate(&self.snapshot, Metric::Response);
            let connect_rate = snapshot.rate(&self.snapshot, Metric::Connect);

            info!(
                "Rate: Request: {:.2} rps Response: {:.2} rps Connect: {:.2} cps",
                request_rate, response_rate, connect_rate
            );

            let request_success =
                snapshot.success_rate(&self.snapshot, Metric::Request, Metric::RequestEx);
            let response_success =
                snapshot.success_rate(&self.snapshot, Metric::Response, Metric::ResponseEx);
            let connect_success =
                snapshot.success_rate(&self.snapshot, Metric::Connect, Metric::ConnectEx);

            info!(
                "Success: Request: {:.2} % Response: {:.2} % Connect: {:.2} %",
                request_success, response_success, connect_success
            );

            let hit_rate =
                snapshot.hitrate(&self.snapshot, Metric::RequestGet, Metric::ResponseHit);

            info!("Hit-rate: {:.2} %", hit_rate);

            if let Some(ref heatmap) = self.connect_heatmap {
                let p25 = heatmap.percentile(0.25).unwrap_or(0);
                let p50 = heatmap.percentile(0.50).unwrap_or(0);
                let p75 = heatmap.percentile(0.75).unwrap_or(0);
                let p90 = heatmap.percentile(0.90).unwrap_or(0);
                let p99 = heatmap.percentile(0.99).unwrap_or(0);
                let p999 = heatmap.percentile(0.999).unwrap_or(0);
                let p9999 = heatmap.percentile(0.9999).unwrap_or(0);
                info!("Connect Latency (us): p25: {} p50: {} p75: {} p90: {} p99: {} p999: {} p9999: {}",
                    p25, p50, p75, p90, p99, p999, p9999
                );
            }

            if let Some(ref heatmap) = self.request_heatmap {
                let p25 = heatmap.percentile(0.25).unwrap_or(0);
                let p50 = heatmap.percentile(0.50).unwrap_or(0);
                let p75 = heatmap.percentile(0.75).unwrap_or(0);
                let p90 = heatmap.percentile(0.90).unwrap_or(0);
                let p99 = heatmap.percentile(0.99).unwrap_or(0);
                let p999 = heatmap.percentile(0.999).unwrap_or(0);
                let p9999 = heatmap.percentile(0.9999).unwrap_or(0);
                info!("Response Latency (us): p25: {} p50: {} p75: {} p90: {} p99: {} p999: {} p9999: {}",
                    p25, p50, p75, p90, p99, p999, p9999
                );
            }

            increment_counter!(&Metric::Window);
            self.snapshot = snapshot.clone();

            if let Some(max_window) = self.config.general().windows() {
                if window >= max_window as u64 {
                    break;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Snapshot {
    counters: HashMap<Metric, u64>,
    _gauges: HashMap<Metric, i64>,
    timestamp: Instant,
    connect_percentiles: Vec<(String, u64)>,
    request_percentiles: Vec<(String, u64)>,
}

impl Snapshot {
    fn new(
        connect_heatmap: Option<&Arc<AtomicHeatmap<u64, AtomicU64>>>,
        request_heatmap: Option<&Arc<AtomicHeatmap<u64, AtomicU64>>>,
    ) -> Self {
        let mut counters = HashMap::new();
        let mut gauges = HashMap::new();
        for metric in Metric::iter() {
            match metric.source() {
                Source::Counter => {
                    let value = get_counter!(&metric).unwrap_or(0);
                    counters.insert(metric, value);
                }
                Source::Gauge => {
                    let value = get_gauge!(&metric).unwrap_or(0);
                    gauges.insert(metric, value);
                }
            }
        }

        let percentiles = vec![
            ("p25", 0.25),
            ("p50", 0.50),
            ("p75", 0.75),
            ("p90", 0.90),
            ("p99", 0.99),
            ("p999", 0.999),
            ("p9999", 0.9999),
        ];

        let mut connect_percentiles = Vec::new();
        if let Some(ref heatmap) = connect_heatmap {
            for (label, value) in &percentiles {
                connect_percentiles
                    .push((label.to_string(), heatmap.percentile(*value).unwrap_or(0)));
            }
        }

        let mut request_percentiles = Vec::new();
        if let Some(ref heatmap) = request_heatmap {
            for (label, value) in &percentiles {
                request_percentiles
                    .push((label.to_string(), heatmap.percentile(*value).unwrap_or(0)));
            }
        }

        Self {
            counters,
            _gauges: gauges,
            timestamp: Instant::now(),
            connect_percentiles,
            request_percentiles,
        }
    }

    fn delta_count(&self, other: &Self, counter: Metric) -> u64 {
        let this = self.counters.get(&counter).unwrap_or(&0);
        let other = other.counters.get(&counter).unwrap_or(&0);
        this - other
    }

    fn rate(&self, other: &Self, counter: Metric) -> f64 {
        let delta = self.delta_count(other, counter) as f64;
        let time = (self.timestamp - other.timestamp).as_secs_f64();
        delta / time
    }

    fn success_rate(&self, other: &Self, total: Metric, error: Metric) -> f64 {
        let total = self.rate(other, total);
        let error = self.rate(other, error);
        if total > 0.0 {
            100.0 - (100.0 * error / total)
        } else {
            100.0
        }
    }

    fn hitrate(&self, other: &Self, total: Metric, hit: Metric) -> f64 {
        let total = self.rate(other, total);
        let hit = self.rate(other, hit);
        if total > 0.0 {
            100.0 * hit / total
        } else {
            0.0
        }
    }

    pub fn human(&self) -> String {
        let mut data = Vec::new();
        for (counter, value) in &self.counters {
            data.push(format!("{}: {}", counter, value));
        }
        for (label, value) in &self.connect_percentiles {
            data.push(format!("connect/latency/{}: {}", label, value));
        }
        for (label, value) in &self.request_percentiles {
            data.push(format!("response/latency/{}: {}", label, value));
        }
        data.sort();
        let mut content = data.join("\n");
        content += "\n";
        content
    }

    pub fn json(&self) -> String {
        let head = "{".to_owned();

        let mut data = Vec::new();
        for (label, value) in &self.counters {
            data.push(format!("\"{}\": {}", label, value));
        }
        for (label, value) in &self.connect_percentiles {
            data.push(format!("\"connect/latency/{}\": {}", label, value));
        }
        for (label, value) in &self.request_percentiles {
            data.push(format!("\"response/latency/{}\": {}", label, value));
        }
        data.sort();
        let body = data.join(",");
        let mut content = head;
        content += &body;
        content += "}";
        content
    }

    pub fn prometheus(&self) -> String {
        let mut data = Vec::new();
        for (counter, value) in &self.counters {
            data.push(format!("{} {}", counter, value));
        }
        for (label, value) in &self.connect_percentiles {
            data.push(format!("connect/latency/{}: {}", label, value));
        }
        for (label, value) in &self.request_percentiles {
            data.push(format!("response/latency/{}: {}", label, value));
        }
        data.sort();
        let mut content = data.join("\n");
        content += "\n";
        let parts: Vec<&str> = content.split('/').collect();
        parts.join("_")
    }
}
