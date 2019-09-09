// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::stats::SimpleRecorder;

use logger::*;
use metrics::{Output, Percentile, Reading};
use std::net::SocketAddr;
use tiny_http::{Method, Response, Server};

pub struct Http {
    recorder: SimpleRecorder,
    server: Server,
    snapshot: Vec<Reading>,
    refreshed: u64,
}

impl Http {
    pub fn new(address: SocketAddr, recorder: SimpleRecorder) -> Self {
        let server = tiny_http::Server::http(address);
        if server.is_err() {
            fatal!("Failed to open {} for HTTP Stats listener", address);
        }
        Self {
            recorder: recorder,
            server: server.unwrap(),
            snapshot: Vec::new(),
            refreshed: 0,
        }
    }

    pub fn run(&mut self) {
        let now = time::precise_time_ns();
        if now - self.refreshed > 500_000_000 {
            self.snapshot = self.recorder.readings();
            self.refreshed = time::precise_time_ns();
        }
        if let Ok(Some(request)) = self.server.try_recv() {
            match request.method() {
                Method::Get => match request.url() {
                    "/" => {
                        debug!("Serving GET on index");
                        let _ = request.respond(Response::from_string(format!(
                            "Welcome to rpc-perf\nVersion: {}",
                            crate::config::VERSION
                        )));
                    }
                    "/metrics.json" | "/vars.json" | "/admin/metrics.json" => {
                        debug!("Serving machine readable stats on /vars");
                        let _ = request.respond(Response::from_string(self.json(false)));
                    }
                    "/vars" => {
                        debug!("Serving human readable stats on /vars");
                        let _ = request.respond(Response::from_string(self.human()));
                    }
                    url => {
                        debug!("GET on non-existent url: {}", url);
                        debug!("Serving machine readable stats on /vars");
                        let _ = request.respond(Response::from_string(self.json(false)));
                    }
                },
                method => {
                    error!("unsupported request method: {}", method);
                    let _ = request.respond(Response::empty(404));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    pub fn human(&self) -> String {
        let mut data = Vec::new();
        for reading in &self.snapshot {
            let label = reading.label();
            let output = reading.output();
            let value = reading.value();
            match output {
                Output::Counter => {
                    data.push(format!("{}/count: {}", label, value));
                }
                Output::Percentile(percentile) => match percentile {
                    Percentile::Minimum => {
                        data.push(format!("{}/minimum/value: {}", label, value));
                    }
                    Percentile::Maximum => {
                        data.push(format!("{}/maximum/value: {}", label, value));
                    }
                    _ => {
                        data.push(format!("{}/histogram/{}: {}", label, percentile, value));
                    }
                },
                Output::MaxPointTime => {
                    // we have point's ns since X and current timespec and current ns sinc X
                    let point_ns = value;
                    let now_timespec = time::get_time();
                    let now_ns = time::precise_time_ns();

                    // find the number of NS in the past for point
                    let delta_ns = now_ns - point_ns;
                    let point_timespec =
                        now_timespec - time::Duration::nanoseconds(delta_ns as i64);

                    // convert to UTC
                    let point_utc = time::at_utc(point_timespec);
                    // calculate offset from the top of the minute
                    let offset = point_utc.tm_sec as u64 * 1_000_000_000 + point_utc.tm_nsec as u64;
                    let offset_ms = (offset as f64 / 1_000_000.0).floor() as usize;
                    data.push(format!("{}/maximum/offset_ms: {}", label, offset_ms));
                }
                _ => {
                    continue;
                }
            }
        }
        data.sort();
        let mut content = data.join("\n");
        content += "\n";
        content
    }

    fn json(&self, pretty: bool) -> String {
        let mut head = "{".to_owned();
        if pretty {
            head += "\n  ";
        }
        let mut data = Vec::new();
        for reading in &self.snapshot {
            let label = reading.label();
            let output = reading.output();
            let value = reading.value();
            match output {
                Output::Counter => {
                    data.push(format!("\"{}/count\": {}", label, value));
                }
                Output::Percentile(percentile) => match percentile {
                    Percentile::Minimum => {
                        data.push(format!("\"{}/minimum/value\": {}", label, value));
                    }
                    Percentile::Maximum => {
                        data.push(format!("\"{}/maximum/value\": {}", label, value));
                    }
                    _ => {
                        data.push(format!("\"{}/histogram/{}\": {}", label, percentile, value));
                    }
                },
                Output::MaxPointTime => {
                    // we have point's ns since X and current timespec and current ns sinc X
                    let point_ns = value;
                    let now_timespec = time::get_time();
                    let now_ns = time::precise_time_ns();

                    // find the number of NS in the past for point
                    let delta_ns = now_ns - point_ns;
                    let point_timespec =
                        now_timespec - time::Duration::nanoseconds(delta_ns as i64);

                    // convert to UTC
                    let point_utc = time::at_utc(point_timespec);
                    // calculate offset from the top of the minute
                    let offset = point_utc.tm_sec as u64 * 1_000_000_000 + point_utc.tm_nsec as u64;
                    let offset_ms = (offset as f64 / 1_000_000.0).floor() as usize;
                    data.push(format!("\"{}/maximum/offset_ms\": {}", label, offset_ms));
                }
                _ => {
                    continue;
                }
            }
        }
        data.sort();
        let body = if pretty {
            data.join(",\n  ")
        } else {
            data.join(",")
        };
        let mut content = head;
        content += &body;
        if pretty {
            content += "\n";
        }
        content += "}";
        content
    }
}
