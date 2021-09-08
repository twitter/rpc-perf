// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_fastmetrics::{MetricsBuilder, Source};
use strum::IntoEnumIterator;
use rustcommon_metrics::{metric, Counter, Gauge};

/// Number of connect attempts.
#[metric(name = "connect")]
pub static CONNECT: Counter = Counter::new();

/// Number of connect errors.
#[metric(name = "connect_ex")]
pub static CONNECT_EX: Counter = Counter::new();

/// Number of connect timeouts.
#[metric(name = "connect_timeout")]
pub static CONNECT_TIMEOUT: Counter = Counter::new();

/// Number of requests sent.
#[metric(name = "request")]
pub static REQUEST: Counter = Counter::new();

/// Number of request errors.
#[metric(name = "request_ex")]
pub static REQUEST_EX: Counter = Counter::new();

/// 'get' requests for hitrate calculations.
#[metric(name = "request_get")]
pub static REQUEST_GET: Counter = Counter::new();

/// 'ok' responses for success rate calculations
#[metric(name = "response")]
pub static RESPONSE: Counter = Counter::new();

/// 'error' responses for success rate calculations
#[metric(name = "response_ex")]
pub static RESPONSE_EX: Counter = Counter::new();

/// 'hit' responses for hitrate calculations
#[metric(name = "response_hit")]
pub static RESPONSE_HIT: Counter = Counter::new();

/// Number of closed connections.
#[metric(name = "close")]
pub static CLOSE: Counter = Counter::new();

/// Number of elapsed windows.
#[metric(name = "window")]
pub static WINDOW: Counter = Counter::new();

/// Number of sessions.
#[metric(name = "session")]
pub static SESSION: Counter = Counter::new();

/// Number of currently open connections.
#[metric(name = "open")]
pub static OPEN: Gauge = Gauge::new();

/// Number of times recv has been called.
#[metric(name = "session_recv")]
pub static SESSION_RECV: Counter = Counter::new();

/// Number of errors calling recv.
#[metric(name = "session_recv_ex")]
pub static SESSION_RECV_EX: Counter = Counter::new();

/// Number of bytes received.
#[metric(name = "session_recv_byte")]
pub static SESSION_RECV_BYTE: Counter = Counter::new();

/// Number of times send has been called.
#[metric(name = "session_send")]
pub static SESSION_SEND: Counter = Counter::new();

/// Number of errors calling send.
#[metric(name = "session_send_ex")]
pub static SESSION_SEND_EX: Counter = Counter::new();

/// Number of bytes sent.
#[metric(name = "session_send_byte")]
pub static SESSION_SEND_BYTE: Counter = Counter::new();

use strum_macros::{AsRefStr, EnumIter};

#[deprecated]
#[derive(Debug, Clone, Copy, AsRefStr, EnumIter, Hash, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum Metric {
    Connect,         // connect attempts
    ConnectEx,       // connect errors
    ConnectTimeout,  // connect timeouts
    Request,         // requests sent
    RequestEx,       // request errors
    RequestGet,      // 'get' keys for hitrate calc
    Response,        // 'ok' responses for success rate calc
    ResponseEx,      // 'error' responses for success rate
    ResponseHit,     // 'hit' responses for hitrate calc
    Close,           // closed connections
    Window,          // elapsed windows
    Session,         // number of sessions
    Open,            // number of open connections
    SessionRecv,     // times recv has been called
    SessionRecvEx,   // number of errors calling recv
    SessionRecvByte, // number of bytes received
    SessionSend,     // times send has been called
    SessionSendEx,   // number of errors calling send
    SessionSendByte, // number of bytes sent
}

impl Into<usize> for Metric {
    fn into(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl rustcommon_fastmetrics::Metric for Metric {
    fn index(&self) -> usize {
        (*self).into()
    }

    fn source(&self) -> Source {
        match self {
            Metric::Open => Source::Gauge,
            _ => Source::Counter,
        }
    }
}

pub fn metrics_init() {
    let metrics: Vec<Metric> = Metric::iter().collect();
    MetricsBuilder::<Metric>::new()
        .metrics(&metrics)
        .build()
        .unwrap();
}
