// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::codec::*;
use crate::config_file::Tls;
use crate::metrics::*;
use crate::session::TcpStream;
use crate::*;
use boring::x509::X509;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rustcommon_heatmap::AtomicHeatmap;
use rustcommon_heatmap::AtomicU64;
use rustcommon_ratelimiter::Ratelimiter;
use std::io::{BufRead, Write};
use std::net::SocketAddr;

use crate::config_file::Protocol;

use boring::ssl::*;
use mio::{Events, Poll, Token};
use slab::Slab;

use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::sync::Arc;

pub struct Worker {
    codec: Box<dyn Codec>,
    connect_queue: VecDeque<(SocketAddr, Option<SslSession>)>,
    connect_ratelimit: Option<Arc<Ratelimiter>>,
    poll: Poll,
    ready_queue: VecDeque<Token>,
    reconnect_ratelimit: Option<Arc<Ratelimiter>>,
    request_ratelimit: Option<Arc<Ratelimiter>>,
    sessions: Slab<Session>,
    tls: Option<SslConnector>,
    connect_heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>,
    request_heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>,
    request_waterfall: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>,
    pipeline: usize,
}

impl Worker {
    pub fn new(config: Arc<Config>) -> Result<Self, std::io::Error> {
        let poll = mio::Poll::new().unwrap();

        let connections = config.connection().poolsize() * config.endpoints().len();
        let sessions = Slab::with_capacity(connections);
        let mut connect_queue = VecDeque::with_capacity(connections);
        let ready_queue = VecDeque::with_capacity(connections);
        let pipeline = config.connection().pipeline();

        // initialize sessions
        for endpoint in config.endpoints() {
            for _ in 0..config.connection().poolsize() {
                connect_queue.push_back((endpoint, None));
            }
        }

        // shuffle connect queue
        let mut tmp: Vec<(SocketAddr, Option<SslSession>)> = connect_queue.drain(0..).collect();
        let mut rng = thread_rng();
        tmp.shuffle(&mut rng);
        for addr in tmp {
            connect_queue.push_back(addr);
        }

        // configure tls connector
        let tls = if let Some(tls_config) = config.tls() {
            ssl_connector(tls_config).expect("bad tls config")
        } else {
            None
        };

        // initialize the codec
        let codec = match config.general().protocol() {
            Protocol::Ping => Box::new(Ping::new(config.clone())) as Box<dyn Codec>,
            Protocol::Echo => Box::new(Echo::new(config.clone())) as Box<dyn Codec>,
            Protocol::Memcache => Box::new(Memcache::new(config.clone())) as Box<dyn Codec>,
            Protocol::Redis | Protocol::RedisInline | Protocol::RedisResp => {
                Box::new(Redis::new(config.clone())) as Box<dyn Codec>
            }
            Protocol::ThriftCache => Box::new(ThriftCache::new(config.clone())) as Box<dyn Codec>,
        };

        // return the worker
        Ok(Worker {
            poll,
            connect_queue,
            connect_ratelimit: None,
            ready_queue,
            reconnect_ratelimit: None,
            request_ratelimit: None,
            sessions,
            tls,
            codec,
            connect_heatmap: None,
            request_heatmap: None,
            request_waterfall: None,
            pipeline,
        })
    }

    /// Controls the total connect rate via an optional shared ratelimiter.
    pub fn set_connect_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.connect_ratelimit = ratelimiter;
    }

    /// Controls the rate at which ready sessions are closed. This can be used
    /// to test server behavior under reconnect load, check for memory leaks,
    /// etc.
    pub fn set_reconnect_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.reconnect_ratelimit = ratelimiter;
    }

    /// Controls the request rate
    pub fn set_request_ratelimit(&mut self, ratelimiter: Option<Arc<Ratelimiter>>) {
        self.request_ratelimit = ratelimiter;
    }

    /// Provide a heatmap for recording connect latency
    pub fn set_connect_heatmap(&mut self, heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>) {
        self.connect_heatmap = heatmap;
    }

    /// Provide a heatmap for recording request latency
    pub fn set_request_heatmap(&mut self, heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>) {
        self.request_heatmap = heatmap;
    }

    /// Provide a heatmap for recording request latencies into the waterfall
    pub fn set_request_waterfall(&mut self, heatmap: Option<Arc<AtomicHeatmap<u64, AtomicU64>>>) {
        self.request_waterfall = heatmap;
    }

    /// Internal function to connect the session
    fn connect(
        &mut self,
        addr: SocketAddr,
        ssl_session: Option<SslSession>,
    ) -> Result<Token, std::io::Error> {
        CONNECT.increment();
        let stream = TcpStream::connect(addr)?;
        let mut session = if let Some(tls) = &self.tls {
            if let Ok(mut connect_config) = tls.configure() {
                if let Some(ssl_session) = ssl_session {
                    unsafe {
                        if connect_config.set_session(&ssl_session).is_err() {
                            return Err(Error::new(ErrorKind::Other, "tls session cache failure"));
                        }
                    }
                }

                match connect_config.connect("localhost", stream) {
                    Ok(stream) => {
                        if stream.ssl().session_reused() {
                            SESSION_REUSE.increment();
                        }
                        Session::tls_with_capacity(stream, 1024, 512 * 1024)
                    }
                    Err(HandshakeError::WouldBlock(stream)) => {
                        if stream.ssl().session_reused() {
                            SESSION_REUSE.increment();
                        }
                        Session::handshaking_with_capacity(stream, 1024, 512 * 1024)
                    }
                    Err(_) => {
                        return Err(Error::new(ErrorKind::Other, "tls failure"));
                    }
                }
            } else {
                return Err(Error::new(ErrorKind::Other, "tls connect config failure"));
            }
        } else {
            Session::plain_with_capacity(stream, 1024, 512 * 1024)
        };

        let entry = self.sessions.vacant_entry();
        let token = Token(entry.key());
        session.set_token(token);
        session.set_timestamp(Instant::now());
        entry.insert(session);
        Ok(token)
    }

    /// Internal function to disconnect the session
    fn disconnect(&mut self, token: Token) -> Result<(), std::io::Error> {
        OPEN.decrement();
        let session = get_session_mut!(self, token)?;
        let _ = session.deregister(&self.poll);
        let peer_addr = session.peer_addr();
        let ssl_session = session.ssl_session();
        session.close();
        if let Ok(addr) = peer_addr {
            self.connect_queue.push_back((addr, ssl_session));
        }
        Ok(())
    }

    /// Check if the session is connecting
    fn is_connecting(&self, token: Token) -> Result<bool, Error> {
        let session = get_session!(self, token)?;
        Ok(session.is_connecting())
    }

    /// Mark the session as connected
    fn connected(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        session.connected();
        Ok(())
    }

    /// Check if the session is handshaking
    fn is_handshaking(&self, token: Token) -> Result<bool, Error> {
        let session = get_session!(self, token)?;
        Ok(session.is_handshaking())
    }

    /// Continue the handshake for the session
    fn handshake(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        session.do_handshake()
    }

    /// Register the token with the event loop
    fn register(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        session.register(&self.poll)
    }

    /// Reregister the token with the event loop
    fn reregister(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        session.reregister(&self.poll)
    }

    /// Get the timestamp for the session, used for latency calculations
    fn timestamp(&mut self, token: Token) -> Result<Instant, Error> {
        let session = get_session_mut!(self, token)?;
        Ok(session.timestamp())
    }

    /// Generate and send a request over the session
    fn send_request(&mut self, token: Token, count: usize) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        for _ in 0..count {
            REQUEST.increment();
            self.codec.encode(session);
        }
        session.set_outstanding(count);
        session.set_timestamp(Instant::now());
        let _ = session.flush();
        if session.write_pending() > 0 {
            self.reregister(token)
        } else {
            Ok(())
        }
    }

    /// Handle reading from the session
    fn do_read(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;

        match session.fill_buf().map(|b| b.len()) {
            Ok(0) => {
                // server hangup
                Err(Error::new(ErrorKind::Other, "server hangup"))
            }
            Ok(_) => {
                // request parsing
                while session.outstanding() > 0 {
                    let response = self.codec.decode(session);
                    match response {
                        Ok(()) => {
                            session.set_outstanding(session.outstanding() - 1);
                            RESPONSE.increment();
                            if let Some(ref heatmap) = self.request_heatmap {
                                let now = Instant::now();
                                let elapsed = now - session.timestamp();
                                let us = elapsed.as_nanos() as u64 / 1_000;
                                heatmap.increment(now, us, 1);
                                if let Some(ref waterfall) = self.request_waterfall {
                                    waterfall.increment(now, elapsed.as_nanos() as u64, 1);
                                }
                            }
                        }
                        Err(e) => match e {
                            ParseError::Incomplete => {
                                return Ok(());
                            }
                            _ => {
                                return Err(Error::from(std::io::ErrorKind::InvalidData));
                            }
                        },
                    }
                }
                self.ready_queue.push_back(token);
                Ok(())
            }
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock => {
                        // spurious read
                        let _ = self.reregister(token);
                        Ok(())
                    }
                    ErrorKind::Interrupted => self.do_read(token),
                    _ => {
                        trace!("error reading for session: {:?} {:?}", session, e);
                        Err(e)
                    }
                }
            }
        }
    }

    /// Handle writing to the session
    fn do_write(&mut self, token: Token) -> Result<(), Error> {
        let session = get_session_mut!(self, token)?;
        if session.write_pending() > 0 {
            session.flush()?;
        }
        Ok(())
    }

    /// Starts the worker event loop. Typically used in a child thread.
    pub fn run(&mut self) {
        let mut events = Events::with_capacity(1024);
        let mut credits = 0;

        loop {
            if let Some((addr, ssl_session)) = self.connect_queue.pop_front() {
                let connect = if let Some(r) = &self.connect_ratelimit {
                    r.try_wait().is_ok()
                } else {
                    true
                };
                if connect {
                    match self.connect(addr, ssl_session) {
                        Ok(token) => {
                            self.register(token).unwrap();
                        }
                        Err(e) => {
                            println!("connect error: {:?} {}", addr, e);
                        }
                    }
                } else {
                    self.connect_queue.push_front((addr, ssl_session));
                }
            }

            if let Some(token) = self.ready_queue.pop_front() {
                let reconnect = if let Some(r) = &self.reconnect_ratelimit {
                    r.try_wait().is_ok()
                } else {
                    false
                };
                if reconnect {
                    let _ = self.disconnect(token);
                } else {
                    if let Some(r) = &self.request_ratelimit {
                        while r.try_wait().is_ok() {
                            credits += 1;
                            if credits == self.pipeline {
                                break;
                            }
                        }
                    } else {
                        credits = self.pipeline;
                    };
                    if credits == self.pipeline {
                        credits = 0;
                        if self.send_request(token, self.pipeline).is_ok() {
                            // yay, we sent a request
                        } else if self.disconnect(token).is_ok() {
                            REQUEST_EX.increment();
                        } else {
                            panic!("this shouldn't happen");
                        }
                    } else {
                        self.ready_queue.push_front(token)
                    }
                }
            }

            let _ = self
                .poll
                .poll(&mut events, Some(std::time::Duration::from_millis(10)));

            for event in &events {
                let token = event.token();

                // handle error events first
                if event.is_error() {
                    if self.is_connecting(token).unwrap() {
                        CONNECT_EX.increment();
                    }
                    // increment_counter!(&Stat::WorkerEventError);
                    let _ = self.disconnect(token);
                    continue;
                }

                // handle handshaking
                if let Ok(true) = self.is_handshaking(token) {
                    if let Err(e) = self.handshake(token) {
                        if e.kind() != ErrorKind::WouldBlock {
                            CONNECT_EX.increment();
                            let _ = self.disconnect(token);
                        }
                    }
                    match self.is_handshaking(token) {
                        Ok(true) => {
                            let _ = self.reregister(token);
                            continue;
                        }
                        Ok(false) => {
                            // finished handshaking
                        }
                        Err(_) => {
                            CONNECT_EX.increment();
                            let _ = self.disconnect(token);
                            continue;
                        }
                    }
                }

                if event.is_readable() && self.do_read(token).is_err() {
                    let _ = self.disconnect(token);
                    continue;
                }

                if event.is_writable() {
                    trace!("got writable for token: {:?}", token);
                    let connecting = self.is_connecting(token).unwrap();
                    let handshaking = self.is_handshaking(token).unwrap();
                    if connecting && !handshaking {
                        self.connected(token).unwrap();
                        OPEN.increment();
                        SESSION.increment();
                        if let Ok(prev) = self.timestamp(token) {
                            if let Some(ref heatmap) = self.connect_heatmap {
                                let now = Instant::now();
                                let elapsed = now - prev;
                                let us = elapsed.as_nanos() as u64 / 1_000;
                                heatmap.increment(now, us, 1);
                            }
                        }
                        self.ready_queue.push_back(token);
                    } else if connecting {
                        OPEN.increment();
                    }
                    if self.do_write(token).is_err() {
                        let _ = self.disconnect(token);
                        continue;
                    }
                }

                let _ = self.reregister(token);
            }
        }
    }
}

pub fn ssl_connector(config: &Tls) -> Result<Option<SslConnector>, std::io::Error> {
    let mut builder = SslConnector::builder(SslMethod::tls_client())?;
    if !config.verify() {
        builder.set_verify(SslVerifyMode::NONE);
    }

    // we use xor here to check if we have an under-specified tls configuration
    if config.private_key().is_some()
        ^ (config.certificate_chain().is_some() || config.certificate().is_some())
    {
        return Err(Error::new(ErrorKind::Other, "incomplete tls configuration"));
    }

    // load the private key
    //
    // NOTE: this is required, so we return `Ok(None)` if it is not specified
    if let Some(f) = config.private_key() {
        builder
            .set_private_key_file(f, SslFiletype::PEM)
            .map_err(|_| Error::new(ErrorKind::Other, "bad private key"))?;
    } else {
        return Ok(None);
    }

    // load the ca file
    //
    // NOTE: this is optional, so we do not return `Ok(None)` when it has not
    // been specified
    if let Some(f) = config.ca_file() {
        builder
            .set_ca_file(f)
            .map_err(|_| Error::new(ErrorKind::Other, "bad ca file"))?;
    }

    match (config.certificate_chain(), config.certificate()) {
        (Some(chain), Some(cert)) => {
            // assume we have the leaf in a standalone file, and the
            // intermediates + root in another file

            // first load the leaf
            builder
                .set_certificate_file(cert, SslFiletype::PEM)
                .map_err(|_| Error::new(ErrorKind::Other, "bad certificate file"))?;

            // append the rest of the chain
            let pem = std::fs::read(chain)
                .map_err(|_| Error::new(ErrorKind::Other, "failed to read certificate chain"))?;
            let chain = X509::stack_from_pem(&pem)
                .map_err(|_| Error::new(ErrorKind::Other, "bad certificate chain"))?;
            for cert in chain {
                builder
                    .add_extra_chain_cert(cert)
                    .map_err(|_| Error::new(ErrorKind::Other, "bad certificate in chain"))?;
            }
        }
        (Some(chain), None) => {
            // assume we have a complete chain: leaf + intermediates + root in
            // one file

            // load the entire chain
            builder
                .set_certificate_chain_file(chain)
                .map_err(|_| Error::new(ErrorKind::Other, "bad certificate chain"))?;
        }
        (None, Some(cert)) => {
            // this will just load the leaf certificate from the file
            builder
                .set_certificate_file(cert, SslFiletype::PEM)
                .map_err(|_| Error::new(ErrorKind::Other, "bad certificate file"))?;
        }
        (None, None) => {
            // if we have neither a chain nor a leaf cert to load, we return no
            // ssl context
            return Ok(None);
        }
    }

    if let Some(size) = config.session_cache() {
        builder.set_session_cache_mode(SslSessionCacheMode::CLIENT);
        builder.set_session_cache_size(size);
    }

    Ok(Some(builder.build()))
}
