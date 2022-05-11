// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_metrics::metric;
pub use rustcommon_metrics::{Counter, Gauge};

#[metric(name = "connect", description = "connect attempts")]
pub static CONNECT: Counter = Counter::new();

#[metric(name = "connect_ex", description = "exceptions when calling connect")]
pub static CONNECT_EX: Counter = Counter::new();

#[metric(name = "connect_timeout", description = "connect timeouts")]
pub static CONNECT_TIMEOUT: Counter = Counter::new();

#[metric(name = "request", description = "requests sent")]
pub static REQUEST: Counter = Counter::new();

#[metric(name = "request_ex", description = "exceptions when sending a request")]
pub static REQUEST_EX: Counter = Counter::new();

#[metric(name = "request_get", description = "get requests sent")]
pub static REQUEST_GET: Counter = Counter::new();

#[metric(name = "response", description = "responses received")]
pub static RESPONSE: Counter = Counter::new();

#[metric(
    name = "response_ex",
    description = "responses that indicated an error"
)]
pub static RESPONSE_EX: Counter = Counter::new();

#[metric(
    name = "response_hit",
    description = "responses that indicated a cache hit"
)]
pub static RESPONSE_HIT: Counter = Counter::new();

/// distribution of response latencies
// #[metric(name = "response_latency")]
// pub static RESPONSE_LATENCY: Relaxed<Heatmap> = Relaxed::new(||
//     Heatmap::new(1_000_000_000, 3, Duration::from_secs(60), Duration::from_secs(1))
// );

#[metric(name = "close", description = "closed connections")]
pub static CLOSE: Counter = Counter::new();

#[metric(name = "window", description = "elapsed windows")]
pub static WINDOW: Counter = Counter::new();

#[metric(name = "session", description = "sessions created")]
pub static SESSION: Counter = Counter::new();

#[metric(name = "open", description = "open connections")]
pub static OPEN: Gauge = Gauge::new();

#[metric(name = "session_recv", description = "session receive attempts")]
pub static SESSION_RECV: Counter = Counter::new();

#[metric(
    name = "session_recv_ex",
    description = "exceptions when calling receive on session"
)]
pub static SESSION_RECV_EX: Counter = Counter::new();

#[metric(
    name = "session_recv_byte",
    description = "bytes received for all sessions"
)]
pub static SESSION_RECV_BYTE: Counter = Counter::new();

#[metric(name = "session_send", description = "session send attempts")]
pub static SESSION_SEND: Counter = Counter::new();

#[metric(
    name = "session_send_ex",
    description = "execptions when calling send on session"
)]
pub static SESSION_SEND_EX: Counter = Counter::new();

#[metric(
    name = "session_send_byte",
    description = "bytes sent for all sessions"
)]
pub static SESSION_SEND_BYTE: Counter = Counter::new();

#[metric(
    name = "session_reuse",
    description = "session reused with abbreviated TLS handshake"
)]
pub static SESSION_REUSE: Counter = Counter::new();
