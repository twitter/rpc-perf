// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_ratelimiter::Refill;

use crate::config::*;

use rustcommon_logger::Level;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct General {
    #[serde(default)]
    protocol: Protocol,
    #[serde(default = "default_interval")]
    interval: usize,
    #[serde(default = "default_windows")]
    windows: Option<usize>,
    #[serde(default = "default_clients")]
    clients: usize,
    #[serde(default = "default_poolsize")]
    poolsize: usize,
    listen: Option<String>,
    admin: Option<String>,
    #[serde(with = "LevelDef")]
    #[serde(default = "default_logging_level")]
    logging: Level,
    endpoints: Option<Vec<String>>,
    request_ratelimit: Option<usize>,
    #[serde(with = "Distribution")]
    #[serde(default = "default_request_distribution")]
    request_distribution: Refill,
    connect_ratelimit: Option<usize>,
    close_rate: Option<usize>,
    tls_key: Option<String>,
    tls_cert: Option<String>,
    tls_ca: Option<String>,
    warmup_hitrate: Option<f64>,
    #[serde(default = "default_tcp_nodelay")]
    tcp_nodelay: bool,
    #[serde(default = "default_request_timeout")]
    request_timeout: usize,
    #[serde(default = "default_connect_timeout")]
    connect_timeout: usize,
    waterfall: Option<String>,
    #[serde(default = "default_soft_timeout")]
    soft_timeout: bool,
}

impl General {
    pub fn listen(&self) -> Option<String> {
        self.listen.clone()
    }

    pub fn set_listen(&mut self, listen: Option<String>) {
        self.listen = listen;
    }

    pub fn admin(&self) -> Option<String> {
        self.admin.clone()
    }

    pub fn set_admin(&mut self, admin: Option<String>) {
        self.admin = admin;
    }

    pub fn clients(&self) -> usize {
        self.clients
    }

    pub fn set_clients(&mut self, clients: usize) {
        self.clients = clients;
    }

    pub fn poolsize(&self) -> usize {
        self.poolsize
    }

    pub fn set_poolsize(&mut self, poolsize: usize) {
        self.poolsize = poolsize;
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
    }

    pub fn interval(&self) -> usize {
        self.interval
    }

    pub fn set_interval(&mut self, seconds: usize) {
        self.interval = seconds;
    }

    pub fn windows(&self) -> Option<usize> {
        self.windows
    }

    pub fn set_windows(&mut self, windows: Option<usize>) {
        self.windows = windows;
    }

    pub fn logging(&self) -> Level {
        self.logging
    }

    pub fn set_logging(&mut self, level: Level) {
        self.logging = level;
    }

    pub fn request_ratelimit(&self) -> Option<usize> {
        self.request_ratelimit
    }

    pub fn set_request_ratelimit(&mut self, per_second: Option<usize>) {
        self.request_ratelimit = per_second;
    }

    pub fn request_distribution(&self) -> Refill {
        self.request_distribution
    }

    pub fn connect_ratelimit(&self) -> Option<usize> {
        self.connect_ratelimit
    }

    pub fn set_request_timeout(&mut self, microseconds: usize) {
        self.request_timeout = microseconds;
    }

    pub fn request_timeout(&self) -> usize {
        self.request_timeout
    }

    pub fn set_connect_timeout(&mut self, microseconds: usize) {
        self.connect_timeout = microseconds;
    }

    pub fn connect_timeout(&self) -> usize {
        self.connect_timeout
    }

    pub fn set_soft_timeout(&mut self, enabled: bool) {
        self.soft_timeout = enabled;
    }

    pub fn soft_timeout(&self) -> bool {
        self.soft_timeout
    }

    pub fn set_connect_ratelimit(&mut self, per_second: Option<usize>) {
        self.connect_ratelimit = per_second;
    }

    pub fn set_close_rate(&mut self, per_second: Option<usize>) {
        self.close_rate = per_second;
    }

    pub fn close_rate(&self) -> Option<usize> {
        self.close_rate
    }

    pub fn endpoints(&self) -> Option<Vec<String>> {
        self.endpoints.clone()
    }

    pub fn set_endpoints(&mut self, endpoints: Option<Vec<String>>) {
        self.endpoints = endpoints;
    }

    pub fn set_tcp_nodelay(&mut self, nodelay: bool) {
        self.tcp_nodelay = nodelay;
    }

    pub fn tcp_nodelay(&self) -> bool {
        self.tcp_nodelay
    }

    #[cfg(feature = "tls")]
    pub fn tls_key(&self) -> Option<String> {
        self.tls_key.clone()
    }

    #[cfg(feature = "tls")]
    pub fn set_tls_key(&mut self, file: Option<String>) {
        self.tls_key = file;
    }

    #[cfg(feature = "tls")]
    pub fn tls_cert(&self) -> Option<String> {
        self.tls_cert.clone()
    }

    #[cfg(feature = "tls")]
    pub fn set_tls_cert(&mut self, file: Option<String>) {
        self.tls_cert = file;
    }

    #[cfg(feature = "tls")]
    pub fn tls_ca(&self) -> Option<String> {
        self.tls_ca.clone()
    }

    #[cfg(feature = "tls")]
    pub fn set_tls_ca(&mut self, file: Option<String>) {
        self.tls_ca = file;
    }

    pub fn set_warmup_hitrate(&mut self, hitrate: Option<f64>) {
        self.warmup_hitrate = hitrate;
    }

    pub fn warmup_hitrate(&self) -> Option<f64> {
        self.warmup_hitrate
    }

    pub fn set_waterfall(&mut self, path: Option<String>) {
        self.waterfall = path;
    }

    pub fn waterfall(&self) -> Option<String> {
        self.waterfall.clone()
    }
}

impl Default for General {
    fn default() -> General {
        General {
            interval: default_interval(),
            windows: default_windows(),
            clients: default_clients(),
            poolsize: default_poolsize(),
            endpoints: None, // no reasonable default endpoints
            listen: None,
            admin: None,
            logging: Level::Info,
            protocol: Default::default(),
            request_ratelimit: None,
            request_distribution: default_request_distribution(),
            connect_ratelimit: None,
            close_rate: None,
            tls_key: None,
            tls_cert: None,
            tls_ca: None,
            warmup_hitrate: None,
            tcp_nodelay: default_tcp_nodelay(),
            request_timeout: default_request_timeout(),
            connect_timeout: default_connect_timeout(),
            waterfall: None,
            soft_timeout: false,
        }
    }
}

fn default_interval() -> usize {
    60
}

fn default_windows() -> Option<usize> {
    Some(5)
}

fn default_clients() -> usize {
    1
}

fn default_poolsize() -> usize {
    1
}

fn default_tcp_nodelay() -> bool {
    false
}

fn default_request_timeout() -> usize {
    200 * MILLISECOND / MICROSECOND
}

fn default_connect_timeout() -> usize {
    200 * MILLISECOND / MICROSECOND
}

fn default_soft_timeout() -> bool {
    false
}

#[derive(Copy, Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Protocol {
    Memcache,
    PelikanRds,
    Ping,
    Echo,
    RedisResp,
    RedisInline,
    ThriftCache,
}

impl Default for Protocol {
    fn default() -> Protocol {
        Protocol::Memcache
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(remote = "Level")]
#[serde(deny_unknown_fields)]
enum LevelDef {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

fn default_logging_level() -> Level {
    Level::Info
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(remote = "Refill")]
#[serde(deny_unknown_fields)]
enum Distribution {
    Smooth,
    Uniform,
    Normal,
}

fn default_request_distribution() -> Refill {
    Refill::Smooth
}
