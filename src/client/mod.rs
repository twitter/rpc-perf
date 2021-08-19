// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::collections::VecDeque;
use std::io::BufRead;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use mio::{Events, Poll, Token};
use rand::prelude::SliceRandom;
use rand::rngs::ThreadRng;
use rand::thread_rng;
use rustcommon_timer::Wheel;
use rustls::ClientConfig;
use rustls::ClientSessionMemoryCache;
use rustls::NoClientSessionStorage;
use slab::Slab;

use crate::codec::*;
use crate::config::Protocol;
use crate::session::{Session, State};
use crate::stats::*;
use crate::*;

pub struct Client {
    codec: Box<dyn Codec>,
    sessions: Slab<Session>,
    config: Arc<Config>,
    ready_queue: VecDeque<usize>,
    connect_queue: VecDeque<SocketAddr>,
    tls_config: Option<Arc<ClientConfig>>,
    metrics: Arc<Metrics>,
    timers: Wheel<usize>,
    last_timeout: Instant,
    events: Option<Events>,
    poll: Poll,
    id: usize,
    connect: Option<Arc<Ratelimiter>>,
    request: Option<Arc<Ratelimiter>>,
    close: Option<Arc<Ratelimiter>>,
}

impl Client {
    pub fn new(
        id: usize,
        config: Arc<Config>,
        connect: Option<Arc<Ratelimiter>>,
        request: Option<Arc<Ratelimiter>>,
        close: Option<Arc<Ratelimiter>>,
        metrics: Arc<Metrics>,
    ) -> Self {
        let codec: Box<dyn Codec> = match config.protocol() {
            Protocol::Echo => Box::new(crate::codec::Echo::new()),
            Protocol::Memcache => Box::new(crate::codec::Memcache::new()),
            Protocol::ThriftCache => Box::new(crate::codec::ThriftCache::new()),
            Protocol::PelikanRds => Box::new(crate::codec::PelikanRds::new()),
            Protocol::Ping => Box::new(crate::codec::Ping::new()),
            Protocol::RedisResp => {
                Box::new(crate::codec::Redis::new(crate::codec::RedisMode::Resp))
            }
            Protocol::RedisInline => {
                Box::new(crate::codec::Redis::new(crate::codec::RedisMode::Inline))
            }
        };

        let tls_config = load_tls_config(&config);

        Self {
            codec,
            sessions: Slab::new(),
            config,
            ready_queue: VecDeque::new(),
            connect_queue: VecDeque::new(),
            metrics,
            tls_config,
            timers: Wheel::<usize>::new(SECOND / MICROSECOND),
            last_timeout: Instant::now(),
            events: None,
            poll: Poll::new().expect("failed to create mio::Poll"),
            id,
            connect,
            request,
            close,
        }
    }

    pub fn add_endpoint(&mut self, addr: &SocketAddr) {
        debug!("client({}) adding endpoint: {}", self.id, addr);
        for _ in 0..self.config.poolsize() {
            self.connect_queue.push_back(*addr);
        }
        self.connect_shuffle();
    }

    fn connect_shuffle(&mut self) {
        let mut tmp: Vec<SocketAddr> = self.connect_queue.drain(0..).collect();
        let mut rng = thread_rng();
        tmp.shuffle(&mut rng);
        for addr in tmp {
            self.connect_queue.push_back(addr);
        }
    }

    fn do_timeouts(&mut self) {
        let last = self.last_timeout;
        let now = Instant::now();
        let ticks =
            (now - last).as_secs() as usize * 1000000 + (now - last).subsec_nanos() as usize / 1000;

        let timeouts = self.timers.tick(ticks);

        for token in timeouts {
            if let Some(state) = self.sessions.get(token).map(|v| v.state()) {
                match state {
                    State::Connecting => {
                        // timeout while connecting
                        self.stat_increment(Stat::ConnectionsTimeout);
                    }
                    State::Reading => {
                        // timeout while reading
                        self.stat_increment(Stat::RequestsTimeout);
                    }
                    _ => {
                        // ignore other timeouts
                        continue;
                    }
                }
                if !self.config.soft_timeout() {
                    let session = self.sessions.remove(token);
                    self.connect_queue.push_back(session.addr());
                }
            }
        }

        self.last_timeout = now;
    }

    fn server_closed(&mut self, token: usize) {
        // server has closed connection
        trace!("server closed: {}", token);
        self.metrics.increment(&Stat::ConnectionsClosed);
        self.metrics.increment(&Stat::ConnectionsServerClosed);
        let mut session = self.sessions.remove(token);
        session.deregister(&self.poll);
        self.connect_queue.push_back(session.addr());
    }

    fn hangup(&mut self, token: usize) {
        if self.sessions.contains(token) {
            self.metrics.increment(&Stat::ConnectionsClosed);
            self.metrics.increment(&Stat::ConnectionsClientClosed);
            let mut session = self.sessions.remove(token);
            session.deregister(&self.poll);
            self.connect_queue.push_back(session.addr());
        }
    }

    fn do_events(&mut self) {
        let mut events = self
            .events
            .take()
            .unwrap_or_else(|| Events::with_capacity(1024));
        self.poll
            .poll(&mut events, Some(Duration::from_millis(1)))
            .unwrap();
        for event in events.iter() {
            let token = event.token();
            if let Some(session) = self.sessions.get_mut(token.0) {
                let read_status = if event.is_readable() {
                    trace!("handle read for: {}", token.0);
                    session.do_read()
                } else {
                    Ok(None)
                };

                let write_status = if event.is_writable() {
                    trace!("handle write for: {}", token.0);
                    session.set_timestamp(Instant::now());
                    session.do_write()
                } else {
                    Ok(None)
                };

                match read_status {
                    Ok(Some(0)) => {
                        self.server_closed(token.0);
                        continue;
                    }
                    Ok(Some(bytes)) => {
                        let start = session.timestamp();
                        // parse response
                        trace!("read {} bytes: {}", bytes, token.0);
                        if let Ok(content) = session.buffer.fill_buf() {
                            trace!("read: {:?}", content);
                            match self.codec.decode(content) {
                                Ok(response) => {
                                    let stop = Instant::now();

                                    self.metrics.heatmap_increment(start, stop);
                                    self.metrics.time_interval(
                                        &Stat::ResponsesLatency,
                                        start,
                                        stop,
                                    );
                                    self.metrics.increment(&Stat::ResponsesTotal);
                                    self.metrics.increment(&Stat::ResponsesOk);

                                    self.ready_queue.push_back(token.0);
                                    session.set_state(State::Writing);

                                    match response {
                                        Response::Hit => {
                                            self.metrics.increment(&Stat::ResponsesHit);
                                        }
                                        Response::Miss => {
                                            self.metrics.increment(&Stat::ResponsesMiss);
                                        }
                                        _ => {}
                                    }
                                }
                                Err(error) => {
                                    if error != Error::Incomplete {
                                        self.metrics.increment(&Stat::ResponsesTotal);
                                        self.metrics.increment(&Stat::ResponsesError);
                                    }

                                    match error {
                                        Error::ChecksumMismatch(a, b) => {
                                            let stop = Instant::now();
                                            let start = session.timestamp();
                                            self.metrics.heatmap_increment(start, stop);
                                            self.metrics.time_interval(
                                                &Stat::ResponsesLatency,
                                                start,
                                                stop,
                                            );
                                            warn!("Response checksum mismatch!");
                                            warn!("Expected: {:?}", a);
                                            warn!("Got: {:?}", b);
                                            self.ready_queue.push_back(token.0);
                                            session.set_state(State::Writing);
                                        }
                                        _ => {
                                            self.hangup(token.0);
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        session.buffer.consume(bytes);
                    }
                    Ok(None) => {
                        // wasn't ready
                        trace!("spurious read: {}", token.0);
                    }
                    Err(_) => {
                        // got some error, close connection
                        self.metrics.increment(&Stat::ResponsesTotal);
                        self.metrics.increment(&Stat::ResponsesError);
                        self.hangup(token.0);
                        continue;
                    }
                }

                match write_status {
                    Ok(Some(bytes)) => {
                        trace!("wrote: {} bytes: {}", bytes, token.0);
                        if session.tx_pending() > 0 {
                            // incomplete write
                            println!("have: {} bytes pending: {}", session.tx_pending(), token.0);
                        } else if bytes > 0 {
                            // completed write
                            self.metrics.increment(&Stat::RequestsDequeued);
                            session.set_state(State::Reading);
                        }
                    }
                    Ok(None) => {
                        trace!("spurious write: {}", token.0);
                    }
                    Err(_) => {
                        // got some error, close connection
                        let session = self.sessions.remove(token.0);
                        self.connect_queue.push_back(session.addr());
                        continue;
                    }
                }

                if session.state() == State::Connecting && !session.is_handshaking() {
                    // increment time interval
                    let stop = Instant::now();
                    let start = session.timestamp();
                    self.metrics
                        .time_interval(&Stat::ConnectionsLatency, start, stop);
                    self.metrics.increment(&Stat::ConnectionsOpened);

                    // finished connecting
                    session.set_state(State::Connected);
                    self.ready_queue.push_back(token.0);
                }
                session.reregister(&self.poll);
            } else {
                // ignore event for unknown session?
            }
        }

        self.events = Some(events);
    }

    fn send_request(&mut self, rng: &mut ThreadRng, token: usize) {
        if let Some(session) = self.sessions.get_mut(token) {
            trace!("send request: {}", token);
            session.set_timestamp(Instant::now());
            self.metrics.increment(&Stat::RequestsEnqueued);
            self.codec.encode(&mut session.buffer, rng);
            session.set_state(State::Writing);
            session.reregister(&self.poll);
        }
    }

    fn do_requests(&mut self, rng: &mut ThreadRng) {
        loop {
            if let Some(token) = self.ready_queue.pop_front() {
                if let Some(ref mut request) = self.request {
                    if request.try_wait().is_ok() {
                        self.send_request(rng, token);
                    } else {
                        self.ready_queue.push_front(token);
                        break;
                    }
                } else {
                    self.send_request(rng, token);
                }
            } else {
                break;
            }
        }
    }

    fn do_hangups(&mut self) {
        if self.close.is_some() {
            loop {
                if let Some(token) = self.ready_queue.pop_front() {
                    trace!("hangup: {}", token);
                    if self.sessions.contains(token) {
                        if self.close.as_ref().unwrap().try_wait().is_ok() {
                            self.hangup(token);
                        } else {
                            self.ready_queue.push_front(token);
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }

    fn connect(&mut self, addr: SocketAddr) {
        let session = self.sessions.vacant_entry();
        let tls = if let Some(ref mut tls_config) = self.tls_config {
            Some(rustls::ClientSession::new(
                &tls_config,
                webpki::DNSNameRef::try_from_ascii_str("localhost").expect("invalid dns name"),
            ))
        } else {
            None
        };
        let start = Instant::now();
        if let Ok(mut s) = Session::new(addr, Token(session.key()), tls) {
            s.set_nodelay(self.config.tcp_nodelay());
            self.metrics.increment(&Stat::ConnectionsTotal);
            if self.tls_config.is_some() {
                s.register(&self.poll);
            } else {
                self.metrics
                    .time_interval(&Stat::ConnectionsLatency, start, Instant::now());
                self.metrics.increment(&Stat::ConnectionsOpened);
                s.register(&self.poll);
                self.ready_queue.push_back(session.key());
            }
            session.insert(s);
        } else {
            self.metrics.increment(&Stat::ConnectionsError);
            self.connect_queue.push_back(addr);
        }
    }

    fn do_connects(&mut self) {
        while let Some(addr) = self.connect_queue.pop_front() {
            trace!("connect: {}", addr);
            if let Some(ref mut connect) = self.connect {
                if connect.try_wait().is_ok() {
                    self.connect(addr);
                } else {
                    self.connect_queue.push_back(addr);
                    break;
                }
            } else {
                self.connect(addr);
            }
        }
    }

    pub fn run(&mut self, rng: &mut ThreadRng) {
        self.do_timeouts();
        self.do_events();
        self.do_connects();
        if self.close.is_some() {
            self.do_hangups();
        }
        self.do_requests(rng);
    }

    fn stat_increment(&self, label: Stat) {
        self.metrics.increment(&label)
    }
}

fn load_tls_config(config: &Arc<Config>) -> Option<Arc<rustls::ClientConfig>> {
    let cert_chain = config.tls_ca();
    let cert = config.tls_cert();
    let key = config.tls_key();
    let session_cache_size = config.tls_session_cache_size();

    if cert_chain.is_some() && cert.is_some() && key.is_some() {
        let mut config = rustls::ClientConfig::new();

        let certificate_chain =
            std::fs::File::open(cert_chain.unwrap()).expect("failed to open cert chain");
        config
            .root_store
            .add_pem_file(&mut std::io::BufReader::new(certificate_chain))
            .expect("failed to load cert chain");

        config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoCertificateVerification {}));

        let cert = std::fs::File::open(cert.unwrap()).expect("failed to open cert");
        let cert = rustls::internal::pemfile::certs(&mut std::io::BufReader::new(cert)).unwrap();

        let key = std::fs::File::open(key.unwrap()).expect("failed to open private key");
        let keys = rustls::internal::pemfile::pkcs8_private_keys(&mut std::io::BufReader::new(key))
            .unwrap();
        assert_eq!(keys.len(), 1);
        let key = keys[0].clone();

        config
            .set_single_client_cert(cert, key)
            .expect("invalid cert or key");

        
        config.session_persistence = match session_cache_size {
            0 => Arc::new(NoClientSessionStorage{}),
            _ => ClientSessionMemoryCache::new(session_cache_size)
        };

        Some(Arc::new(config))
    } else if cert_chain.is_none() && cert.is_none() && key.is_none() {
        None
    } else {
        fatal!("Invalid TLS configuration");
    }
}

pub struct NoCertificateVerification {}

impl rustls::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _roots: &rustls::RootCertStore,
        _presented_certs: &[rustls::Certificate],
        _dns_name: webpki::DNSNameRef<'_>,
        _ocsp: &[u8],
    ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        Ok(rustls::ServerCertVerified::assertion())
    }
}
