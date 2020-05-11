// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::ClientConfig;

use logger::*;
use tiny_http::{Method, Response, Server};

use std::net::SocketAddr;

pub(crate) struct Http {
    client_config: ClientConfig,
    server: Server,
}

impl Http {
    pub fn new(address: SocketAddr, client_config: ClientConfig) -> Self {
        let server = tiny_http::Server::http(address);
        if server.is_err() {
            fatal!("Failed to open {} for HTTP Admin listener", address);
        }
        Self {
            client_config,
            server: server.unwrap(),
        }
    }

    pub fn run(&mut self) {
        if let Ok(Some(mut request)) = self.server.try_recv() {
            match request.method() {
                Method::Get => match request.url() {
                    "/" => {
                        debug!("Serving GET on index");
                        let _ = request.respond(Response::from_string(format!(
                            "rpc-perf\nVersion: {}\nAdmin port",
                            crate::config::VERSION
                        )));
                    }
                    "/ratelimit/request" => {
                        if self.client_config.request_ratelimiter.is_some() {
                            let _ = request.respond(Response::from_string(format!(
                                "{}",
                                self.client_config
                                    .request_ratelimiter
                                    .as_ref()
                                    .unwrap()
                                    .rate()
                            )));
                        } else {
                            let _ = request.respond(Response::from_string("None".to_string()));
                        }
                    }
                    url => {
                        debug!("GET on non-existent url: {}", url);
                        let _ = request.respond(Response::empty(404));
                    }
                },
                Method::Put => match request.url() {
                    "/ratelimit/request" => {
                        let mut content = String::new();
                        request.as_reader().read_to_string(&mut content).unwrap();
                        if let Ok(rate) = content.parse() {
                            if let Some(ref ratelimiter) = self.client_config.request_ratelimiter {
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
                    error!("unsupported request method: {}", method);
                    let _ = request.respond(Response::empty(404));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
