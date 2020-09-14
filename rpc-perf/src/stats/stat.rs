// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_metrics::{AtomicU32, AtomicU64, Source, Statistic};
use strum_macros::{EnumIter, EnumString, IntoStaticStr};

#[derive(
    Clone,
    Copy,
    Debug,
    EnumIter,
    EnumString,
    Eq,
    IntoStaticStr,
    PartialEq,
    Hash,
)]
pub enum Stat {
    #[strum(serialize = "window")]
    Window,
    #[strum(serialize = "requests/enqueued")]
    RequestsEnqueued,
    #[strum(serialize = "requests/dequeued")]
    RequestsDequeued,
    #[strum(serialize = "requests/error")]
    RequestsError,
    #[strum(serialize = "requests/timeout")]
    RequestsTimeout,
    #[strum(serialize = "connections/total")]
    ConnectionsTotal,
    #[strum(serialize = "connections/opened")]
    ConnectionsOpened,
    #[strum(serialize = "connections/closed")]
    ConnectionsClosed,
    #[strum(serialize = "connections/error")]
    ConnectionsError,
    #[strum(serialize = "connections/closed/client")]
    ConnectionsClientClosed,
    #[strum(serialize = "connections/closed/server")]
    ConnectionsServerClosed,
    #[strum(serialize = "connections/timeout")]
    ConnectionsTimeout,
    #[strum(serialize = "responses/total")]
    ResponsesTotal,
    #[strum(serialize = "responses/ok")]
    ResponsesOk,
    #[strum(serialize = "responses/error")]
    ResponsesError,
    #[strum(serialize = "responses/hit")]
    ResponsesHit,
    #[strum(serialize = "responses/miss")]
    ResponsesMiss,
    #[strum(serialize = "commands/create")]
    CommandsCreate,
    #[strum(serialize = "commands/delete")]
    CommandsDelete,
    #[strum(serialize = "commands/find")]
    CommandsFind,
    #[strum(serialize = "commands/get")]
    CommandsGet,
    #[strum(serialize = "commands/len")]
    CommandsLen,
    #[strum(serialize = "commands/push")]
    CommandsPush,
    #[strum(serialize = "commands/range")]
    CommandsRange,
    #[strum(serialize = "commands/remove")]
    CommandsRemove,
    #[strum(serialize = "commands/set")]
    CommandsSet,
    #[strum(serialize = "commands/trim")]
    CommandsTrim,
    #[strum(serialize = "commands/truncate")]
    CommandsTruncate,
    #[strum(serialize = "key/size")]
    KeySize,
    #[strum(serialize = "value/size")]
    ValueSize,
}

impl Statistic<AtomicU64, AtomicU32> for Stat {
    fn name(&self) -> &str {
        (*self).into()
    }

    fn source(&self) -> Source {
        match self {
            Self::KeySize | Self::ValueSize | Self::ConnectionsOpened | Self::ResponsesTotal => Source::Distribution,
            _ => Source::Counter,
        }
    }
}
