// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::metrics::Metric;
use crate::metrics::*;
use std::time::Instant;

use boring::ssl::*;

use bytes::{Buf, BytesMut};
use mio::net::TcpStream;
use mio::{Interest, Poll, Token};

use std::borrow::Borrow;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::SocketAddr;

pub enum Stream {
    Plain(TcpStream),
    Tls(SslStream<TcpStream>),
    Handshaking(MidHandshakeSslStream<TcpStream>),
}

/// A `Session` is the complete state of a TCP stream
pub struct Session {
    token: Token,
    addr: SocketAddr,
    stream: Option<Stream>,
    pub read_buffer: BytesMut,
    pub write_buffer: BytesMut,
    tmp_buffer: [u8; 1024],
    connected: bool,
    timestamp: Instant,
}

impl Session {
    // Create a new `Session`
    pub fn new(addr: SocketAddr) -> Self {
        #[cfg(feature = "metrics")]
        let _ = metrics.increment_counter(Stat::TcpAccept, 1);
        Self {
            token: Token(0),
            addr,
            stream: None,
            read_buffer: BytesMut::with_capacity(1024),
            write_buffer: BytesMut::with_capacity(1024),
            tmp_buffer: [0; 1024],
            connected: false,
            timestamp: Instant::now(),
        }
    }

    pub fn connected(&mut self) {
        if !self.connected {
            increment_gauge!(&Metric::Open);
            OPEN.increment();
            increment_counter!(&Metric::Session);
            SESSION.increment();
            self.connected = true;
        }
    }

    pub fn is_connecting(&self) -> bool {
        !self.connected
    }

    pub fn connect(
        &mut self,
        tls: Option<&SslConnector>,
        nodelay: bool,
    ) -> Result<(), std::io::Error> {
        self.timestamp = Instant::now();

        let stream = TcpStream::connect(self.addr)?;
        stream.set_nodelay(nodelay)?;

        if let Some(tls) = tls {
            match tls.connect("localhost", stream) {
                Ok(s) => {
                    self.stream = Some(Stream::Tls(s));
                    Ok(())
                }
                Err(HandshakeError::WouldBlock(s)) => {
                    self.stream = Some(Stream::Handshaking(s));
                    Ok(())
                }
                Err(_) => Err(Error::new(ErrorKind::Other, "tls failure")),
            }
        } else {
            self.stream = Some(Stream::Plain(stream));
            Ok(())
        }
    }

    /// Register the `Session` with the event loop
    pub fn register(&mut self, poll: &Poll) -> Result<(), std::io::Error> {
        let interest = self.readiness();
        match &mut self.stream {
            Some(Stream::Plain(s)) => poll.registry().register(s, self.token, interest),
            Some(Stream::Tls(s)) => poll.registry().register(s.get_mut(), self.token, interest),
            Some(Stream::Handshaking(s)) => {
                poll.registry().register(s.get_mut(), self.token, interest)
            }
            _ => Err(Error::new(ErrorKind::Other, "session has no stream")),
        }
    }

    /// Deregister the `Session` from the event loop
    pub fn deregister(&mut self, poll: &Poll) -> Result<(), std::io::Error> {
        match &mut self.stream {
            Some(Stream::Plain(s)) => poll.registry().deregister(s),
            Some(Stream::Tls(s)) => poll.registry().deregister(s.get_mut()),
            Some(Stream::Handshaking(s)) => poll.registry().deregister(s.get_mut()),
            _ => Err(Error::new(ErrorKind::Other, "session has no stream")),
        }
    }

    /// Reregister the `Session` with the event loop
    pub fn reregister(&mut self, poll: &Poll) -> Result<(), std::io::Error> {
        let interest = self.readiness();
        match &mut self.stream {
            Some(Stream::Plain(s)) => poll.registry().reregister(s, self.token, interest),
            Some(Stream::Tls(s)) => poll
                .registry()
                .reregister(s.get_mut(), self.token, interest),
            Some(Stream::Handshaking(s)) => {
                poll.registry()
                    .reregister(s.get_mut(), self.token, interest)
            }
            _ => Err(Error::new(ErrorKind::Other, "session has no stream")),
        }
    }

    /// Reads from the stream into the session buffer
    pub fn read(&mut self) -> Result<Option<usize>, std::io::Error> {
        increment_counter!(&Metric::SessionRecv);
        SESSION_RECV.increment();
        let mut total_bytes = 0;
        loop {
            let read_result = match &mut self.stream {
                Some(Stream::Plain(s)) => s.read(&mut self.tmp_buffer),
                Some(Stream::Tls(s)) => s.read(&mut self.tmp_buffer),
                Some(Stream::Handshaking(_)) => {
                    return Ok(None);
                }
                _ => {
                    return Err(Error::new(ErrorKind::Other, "session has no stream"));
                }
            };
            match read_result {
                Ok(0) => {
                    // server terminated connection, but we may have read some
                    // data already.
                    break;
                }
                Ok(bytes) => {
                    increment_counter_by!(&Metric::SessionRecvByte, bytes as u64);
                    SESSION_RECV_BYTE.add(bytes as _);
                    self.read_buffer
                        .extend_from_slice(&self.tmp_buffer[0..bytes]);
                    total_bytes += bytes;
                    if bytes < self.tmp_buffer.len() {
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        if total_bytes == 0 {
                            return Ok(None);
                        } else {
                            break;
                        }
                    } else {
                        trace!("error reading from session");
                        increment_counter!(&Metric::SessionRecvEx);
                        SESSION_RECV_EX.increment();
                        return Err(e);
                    }
                }
            }
        }
        Ok(Some(total_bytes))
    }

    /// Flush the session buffer to the stream
    pub fn flush(&mut self) -> Result<Option<usize>, std::io::Error> {
        self.timestamp = Instant::now();
        increment_counter!(&Metric::SessionSend);
        SESSION_SEND.increment();
        let write_result = match &mut self.stream {
            Some(Stream::Plain(s)) => s.write(&self.write_buffer.borrow()),
            Some(Stream::Tls(s)) => s.write(&self.write_buffer.borrow()),
            Some(Stream::Handshaking(_)) => {
                return Ok(None);
            }
            _ => {
                return Err(Error::new(ErrorKind::Other, "session has no stream"));
            }
        };
        match write_result {
            Ok(0) => Ok(Some(0)),
            Ok(bytes) => {
                increment_counter_by!(&Metric::SessionSendByte, bytes as u64);
                SESSION_SEND_BYTE.add(bytes as _);
                self.write_buffer.advance(bytes);
                Ok(Some(bytes))
            }
            Err(e) => {
                increment_counter!(&Metric::SessionSendEx);
                SESSION_SEND_EX.increment();
                Err(e)
            }
        }
    }

    /// Set the token which is used with the event loop
    pub fn set_token(&mut self, token: Token) {
        self.token = token;
    }

    /// Get the set of readiness events the session is waiting for
    fn readiness(&self) -> Interest {
        if self.write_buffer.is_empty() && self.connected {
            Interest::READABLE
        } else {
            Interest::READABLE | Interest::WRITABLE
        }
    }

    pub fn is_handshaking(&self) -> bool {
        matches!(self.stream, Some(Stream::Handshaking(_)))
    }

    pub fn do_handshake(&mut self) -> Result<(), std::io::Error> {
        if let Some(Stream::Handshaking(stream)) = self.stream.take() {
            let ret;
            let result = stream.handshake();
            self.stream = match result {
                Ok(established) => {
                    ret = Ok(());
                    Some(Stream::Tls(established))
                }
                Err(HandshakeError::WouldBlock(handshaking)) => {
                    ret = Err(Error::new(ErrorKind::WouldBlock, "handshake would block"));
                    Some(Stream::Handshaking(handshaking))
                }
                Err(e) => {
                    debug!("handshake error: {}", e);
                    ret = Err(Error::new(ErrorKind::Other, "handshaking error"));
                    None
                }
            };
            ret
        } else {
            panic!("corrupted session");
        }
    }

    pub fn close(&mut self) {
        trace!("closing session");
        if self.connected {
            decrement_gauge!(&Metric::Open);
            OPEN.decrement();
            increment_counter!(&Metric::Close);
            CLOSE.increment();
            self.connected = false;
        }
        self.read_buffer.clear();
        self.write_buffer.clear();
        if let Some(stream) = self.stream.take() {
            self.stream = match stream {
                Stream::Plain(s) => {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                    None
                }
                Stream::Tls(mut s) => {
                    // TODO(bmartin): session resume requires that a full graceful
                    // shutdown occurs
                    let _ = s.shutdown();
                    None
                }
                Stream::Handshaking(mut s) => {
                    // since we don't have a fully established session, just
                    // shutdown the underlying tcp stream
                    let _ = s.get_mut().shutdown(std::net::Shutdown::Both);
                    None
                }
            }
        }
    }

    pub fn read_pending(&self) -> usize {
        self.read_buffer.len()
    }

    pub fn write_pending(&self) -> usize {
        self.write_buffer.len()
    }

    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}
