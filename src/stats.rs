//  rpc-perf - RPC Performance Testing
//  Copyright 2015 Twitter, Inc
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

extern crate heatmap;
extern crate histogram;
extern crate waterfall;
extern crate shuteye;
extern crate time;
extern crate tiny_http;

use std::collections::HashMap;
use std::fmt;
use std::net::ToSocketAddrs;
use std::process;
use std::sync::mpsc;

use heatmap::{Heatmap, HeatmapConfig};
use histogram::{Histogram, HistogramConfig};
use tiny_http::{Server, Response, Request};
use waterfall::Waterfall;

const ONE_MILISECOND: i64 = 1_000_000;
const ONE_SECOND: u64 = 1_000_000_000;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Counter {
    Total,
    Ok,
    Error,
    Hit,
    Miss,
    Closed,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Gauge {
    Percentile50,
    Percentile90,
    Percentile99,
    Percentile999,
    Percentile9999,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Error,
    Hit,
    Miss,
    Ok,
    Closed,
}

pub struct Counters {
    counts: HashMap<Counter, u64>,
}

pub struct Gauges {
    gauges: HashMap<Gauge, u64>,
}

#[derive(Clone)]
pub struct Stat {
    pub start: u64,
    pub stop: u64,
    pub status: Status,
}

pub struct Receiver {
    queue: mpsc::Receiver<Stat>,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::Ok => write!(f, "ok"),
            Status::Error => write!(f, "error"),
            Status::Hit => write!(f, "hit"),
            Status::Miss => write!(f, "miss"),
            Status::Closed => write!(f, "closed"),
        }
    }
}

impl fmt::Display for Gauge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Gauge::Percentile50 => write!(f, "p50"),
            Gauge::Percentile90 => write!(f, "p90"),
            Gauge::Percentile99 => write!(f, "p99"),
            Gauge::Percentile999 => write!(f, "p999"),
            Gauge::Percentile9999 => write!(f, "p9999"),
        }
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Counter::Total => write!(f, "total"),
            Counter::Ok => write!(f, "ok"),
            Counter::Error => write!(f, "error"),
            Counter::Hit => write!(f, "hit"),
            Counter::Miss => write!(f, "miss"),
            Counter::Closed => write!(f, "closed"),
        }
    }
}

impl Counters {
    pub fn new() -> Counters {
        Counters { counts: HashMap::new() }
    }

    pub fn increment(&mut self, counter: Counter) {
        self.add(counter, 1);
    }

    pub fn add(&mut self, counter: Counter, count: u64) {
        if let Some(c) = self.counts.get_mut(&counter) {
            *c += count;
            return;
        }
        self.counts.insert(counter, count);
    }

    pub fn clear(&mut self) {
        self.counts = HashMap::new();
    }

    pub fn get(&self, counter: Counter) -> u64 {
        if let Some(c) = self.counts.get(&counter) {
            return *c;
        }
        0
    }
}

impl Gauges {
    pub fn new() -> Gauges {
        Gauges { gauges: HashMap::new() }
    }

    pub fn set(&mut self, gauge: Gauge, value: u64) {
        if let Some(c) = self.gauges.get_mut(&gauge) {
            *c = value;
            return;
        }
        self.gauges.insert(gauge, value);
    }
}

fn response_stats(counters: &Counters) {
    info!("Responses: {} Ok: {} Error: {} Closed: {} Hit: {} Miss: {} ",
                          counters.get(Counter::Total),
                          counters.get(Counter::Ok),
                          counters.get(Counter::Error),
                          counters.get(Counter::Closed),
                          counters.get(Counter::Hit),
                          counters.get(Counter::Miss),
                        );
}

fn pretty_percentile(histogram: &Histogram, percentile: f64) -> String {
    match histogram.percentile(percentile) {
        Ok(v) => format!("{} ns", v),
        Err(e) => e.to_owned(),
    }
}

fn histogram_stats(histogram: &Histogram) {
    info!("Percentiles: p50: {} p90: {} p99: {} p999: {} p9999: {}",
        pretty_percentile(histogram, 50.0),
        pretty_percentile(histogram, 90.0),
        pretty_percentile(histogram, 99.0),
        pretty_percentile(histogram, 99.9),
        pretty_percentile(histogram, 99.99),
    );
}

fn counter_percent(c: &Counters, a: Counter, b: Counter) -> f64 {
    let a = c.get(a) as f64;
    let b = c.get(b) as f64;

    let t = a + b;

    if t > 0.0 {
        return 100_f64 * a / t;
    }
    0.0
}

fn counter_rate(c: &Counters, time: u64, counter: Counter) -> f64 {
    (ONE_SECOND * c.get(counter)) as f64 / time as f64
}

fn start_listener(listen: Option<String>) -> Option<Server> {
    if let Some(ref l) = listen {
        let http_socket = l.to_socket_addrs().unwrap().next().unwrap();

        debug!("stats: starting HTTP listener");
        return Some(Server::http(http_socket).unwrap());
    }
    None
}

fn try_handle_http(server: &Option<Server>, mut histogram: &mut Histogram, gauges: &Gauges, counters: &Counters) {
    if let Some(ref s) = *server {
        if let Ok(Some(request)) = s.try_recv() {
            debug!("stats: handle http request");
            handle_http(request, &mut histogram, &gauges, &counters);
        }
    }
}

fn handle_http(request: Request, histogram: &mut Histogram, gauges: &Gauges, counters: &Counters) {
    let mut output = "".to_owned();

    match request.url() {
        "/histogram" => {
            for bucket in histogram {
                if bucket.count() > 0 {
                    output = output + &format!("{} {}\n", bucket.value(), bucket.count());
                }
            }
        }
        "/vars" => {
            for (stat, value) in &counters.counts {
                output = output + &format!("{}: {}\n", stat, value);
            }
            for (stat, value) in &gauges.gauges {
                output = output + &format!("{}: {}\n", stat, value);
            }
        }
        _ => {
            output = output + "{";
            for (stat, value) in &counters.counts {
                output = output + &format!("\"{}\":{},", stat, value);
            }
            for (stat, value) in &gauges.gauges {
                output = output + &format!("\"{}\":{},", stat, value);
            }
            let _ = output.pop();
            output = output + "}";
        }
    }

    let response = Response::from_string(output);
    let _ = request.respond(response);
}

impl Receiver {
    pub fn new(queue: mpsc::Receiver<Stat>) -> Receiver {
        Receiver { queue: queue }
    }

    pub fn run(&self,
               duration: usize,
               windows: usize,
               trace: Option<String>,
               waterfall: Option<String>,
               max_closed: usize,
               listen: Option<String>) {

        debug!("stats: initialize datastructures");
        let mut histogram_config = HistogramConfig::new();
        histogram_config.precision(4).max_value(60 as u64 * ONE_SECOND);
        let mut histogram = Histogram::configured(histogram_config).unwrap();
        let mut http_histogram = histogram.clone();

        let mut heatmap_config = HeatmapConfig::new();
        heatmap_config.precision(2).max_value(ONE_SECOND);
        heatmap_config.slice_duration(ONE_SECOND as u64).num_slices((duration * windows));
        let mut heatmap = Heatmap::configured(heatmap_config).unwrap();

        let mut printed_at = time::precise_time_ns();
        let mut window_counters = Counters::new();
        let mut global_counters = Counters::new();
        let mut gauges = Gauges::new();
        let mut window = 0;
        let mut closed = 0;
        let mut warmup = true;

        let server = start_listener(listen);

        debug!("stats: collection ready");
        loop {
            match self.queue.try_recv() {
                Ok(result) => {
                    match result.status {
                        Status::Ok => {
                            window_counters.increment(Counter::Ok);
                        }
                        Status::Hit => {
                            window_counters.increment(Counter::Ok);
                            window_counters.increment(Counter::Hit);
                        }
                        Status::Miss => {
                            window_counters.increment(Counter::Ok);
                            window_counters.increment(Counter::Miss);
                        }
                        Status::Error => {
                            window_counters.increment(Counter::Error);
                        }
                        Status::Closed => {
                            closed += 1;
                            window_counters.increment(Counter::Closed);
                        }
                    }
                    window_counters.increment(Counter::Total);
                    let _ = histogram.increment(result.stop - result.start);
                    let _ = heatmap.increment(result.start, result.stop - result.start);
                }
                Err(_) => {
                    shuteye::sleep(shuteye::Timespec::from_nano(ONE_MILISECOND).unwrap());
                }
            }

            try_handle_http(&server, &mut http_histogram, &gauges, &global_counters);

            if closed == max_closed {
                error!("all connections have closed!");
                process::exit(1);
            }

            let now = time::precise_time_ns();

            if now - printed_at >= (duration as u64 * ONE_SECOND) {
                if warmup {
                    info!("-----");
                    info!("Warmup complete");
                    warmup = false;
                    let _ = heatmap.clear();
                } else {
                    let rate = counter_rate(&window_counters, (now - printed_at), Counter::Total);
                    let success_rate = counter_percent(&window_counters,
                                                       Counter::Ok,
                                                       Counter::Error);
                    let hit_rate = counter_percent(&window_counters, Counter::Hit, Counter::Miss);
                    info!("-----");
                    info!("Window: {}", window);
                    response_stats(&window_counters);
                    info!("Rate: {:.*} rps Success: {:.*} % Hitrate: {:.*} %",
                          2,
                          rate,
                          2,
                          success_rate,
                          2,
                          hit_rate);
                    info!("Latency: min: {} ns max: {} ns",
	                        histogram.minimum().unwrap_or(0),
	                        histogram.maximum().unwrap_or(0),
	                    );
                    histogram_stats(&histogram);
                }

                // set gauges to match window stats
                gauges.set(Gauge::Percentile50, histogram.percentile(50.0).unwrap_or(0));
                gauges.set(Gauge::Percentile90, histogram.percentile(90.0).unwrap_or(0));
                gauges.set(Gauge::Percentile99, histogram.percentile(99.0).unwrap_or(0));
                gauges.set(Gauge::Percentile999,
                           histogram.percentile(99.9).unwrap_or(0));
                gauges.set(Gauge::Percentile9999,
                           histogram.percentile(99.99).unwrap_or(0));

                // increment global counters
                for c in [Counter::Total,
                          Counter::Ok,
                          Counter::Error,
                          Counter::Hit,
                          Counter::Miss,
                          Counter::Closed]
                             .into_iter() {
                    global_counters.add(c.clone(), window_counters.get(c.clone()));
                }

                http_histogram = histogram.clone();

                // clear the window stats
                let _ = histogram.clear();
                window_counters.clear();

                window += 1;
                printed_at = now;
                if window > windows || closed == max_closed {
                    if let Some(file) = trace {
                        debug!("stats: saving trace file");
                        heatmap.save(file);
                    }
                    if let Some(file) = waterfall {
                        debug!("stats: saving waterfall render");
                        let mut waterfall = Waterfall { heatmap: heatmap };
                        waterfall.render_png(file);
                    }
                    break;
                }
            }
        }
    }
}
