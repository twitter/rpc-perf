use metrics::Source;
use metrics::Statistic;

#[derive(Copy, Clone)]
pub enum Stat {
    Window,
    RequestsEnqueued,
    RequestsDequeued,
    RequestsError,
    RequestsTimeout,
    ConnectionsTotal,
    ConnectionsOpened,
    ConnectionsClosed,
    ConnectionsError,
    ConnectionsClientClosed,
    ConnectionsServerClosed,
    ConnectionsTimeout,
    ResponsesTotal,
    ResponsesOk,
    ResponsesError,
    ResponsesHit,
    ResponsesMiss,
    CommandsCreate,
    CommandsDelete,
    CommandsFind,
    CommandsGet,
    CommandsLen,
    CommandsPush,
    CommandsRange,
    CommandsRemove,
    CommandsSet,
    CommandsTrim,
    CommandsTruncate,
    KeySize,
    ValueSize,
}

impl Statistic for Stat {
    fn name(&self) -> &str {
        match self {
            Self::CommandsCreate => "commands/create",
            Self::CommandsDelete => "commands/delete",
            Self::CommandsFind => "commands/find",
            Self::CommandsGet => "commands/get",
            Self::CommandsLen => "commands/len",
            Self::CommandsPush => "commands/push",
            Self::CommandsRange => "commands/range",
            Self::CommandsRemove => "commands/remove",
            Self::CommandsSet => "commands/set",
            Self::CommandsTrim => "commands/trim",
            Self::CommandsTruncate => "commands/truncate",
            Self::KeySize => "keys/size",
            Self::ValueSize => "values/size",
            Self::Window => "window",
            Self::RequestsEnqueued => "requests/enqueued",
            Self::RequestsDequeued => "requests/dequeued",
            Self::RequestsError => "requests/error",
            Self::RequestsTimeout => "requests/timeout",
            Self::ConnectionsTotal => "connections/total",
            Self::ConnectionsOpened => "connections/opened",
            Self::ConnectionsClosed => "connections/closed/total",
            Self::ConnectionsError => "connections/error",
            Self::ConnectionsClientClosed => "connections/closed/client",
            Self::ConnectionsServerClosed => "connections/closed/server",
            Self::ConnectionsTimeout => "connections/timeout",
            Self::ResponsesTotal => "responses/total",
            Self::ResponsesOk => "responses/ok",
            Self::ResponsesError => "responses/error",
            Self::ResponsesHit => "responses/hit",
            Self::ResponsesMiss => "responses/miss",
        }
    }

    fn source(&self) -> Source {
        match self {
            Self::KeySize => Source::Distribution,
            Self::ValueSize => Source::Distribution,
            Self::ConnectionsOpened => Source::TimeInterval,
            Self::ResponsesTotal => Source::TimeInterval,
            _ => Source::Counter,
        }
    }
}