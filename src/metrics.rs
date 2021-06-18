// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_fastmetrics::{MetricsBuilder, Source};
use strum::IntoEnumIterator;

use strum_macros::{AsRefStr, EnumIter};

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
