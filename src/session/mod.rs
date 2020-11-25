// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::net::SocketAddr;
use std::time::Instant;

use mio::net::TcpStream;
use mio::{Interest, Poll, Token};
use rustcommon_buffer::Buffer;
use rustls::ClientSession;
use rustls::Session as TlsSession;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Connecting,
    Connected,
    Reading,
    Writing,
}

pub struct Session {
    addr: SocketAddr,
    pub(crate) buffer: Buffer,
    stream: TcpStream,
    tls: Option<ClientSession>,
    state: State,
    token: Token,
    timestamp: Instant,
}

impl Session {
    pub fn new(addr: SocketAddr, token: Token, tls: Option<ClientSession>) -> Result<Self, ()> {
        if let Ok(stream) = TcpStream::connect(addr) {
            let state = if tls.is_some() {
                State::Connecting
            } else {
                State::Connected
            };
            Ok(Self {
                addr,
                buffer: Buffer::with_capacity(1024, 1024),
                stream,
                tls,
                token,
                state,
                timestamp: Instant::now(),
            })
        } else {
            Err(())
        }
    }

    pub fn set_nodelay(&mut self, nodelay: bool) {
        let _ = self.stream.set_nodelay(nodelay);
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn is_handshaking(&self) -> bool {
        if let Some(ref tls) = self.tls {
            tls.is_handshaking()
        } else {
            false
        }
    }

    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: Instant) {
        self.timestamp = timestamp;
    }

    pub fn do_read(&mut self) -> Result<Option<usize>, std::io::Error> {
        if let Some(ref mut tls) = self.tls {
            match tls.read_tls(&mut self.stream) {
                Err(e) => Err(e),
                Ok(0) => Ok(Some(0)),
                Ok(_) => {
                    if tls.process_new_packets().is_err() {
                        Ok(None)
                    } else {
                        match self.buffer.read_from(tls) {
                            Ok(Some(0)) | Ok(None) => Ok(None),
                            Ok(Some(bytes)) => Ok(Some(bytes)),
                            Err(e) => Err(e),
                        }
                    }
                }
            }
        } else {
            self.buffer.read_from(&mut self.stream)
        }
    }

    pub fn do_write(&mut self) -> Result<Option<usize>, std::io::Error> {
        if let Some(ref mut tls) = self.tls {
            match tls.write_tls(&mut self.stream) {
                Ok(_) => {
                    if self.buffer.write_pending() > 0 {
                        self.buffer.write_to(tls)
                    } else {
                        Ok(None)
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            self.buffer.write_to(&mut self.stream)
        }
    }

    pub fn tx_pending(&self) -> usize {
        self.buffer.write_pending()
    }

    pub fn interests(&self) -> Interest {
        if let Some(ref tls) = self.tls {
            let r = tls.wants_read();
            let w = tls.wants_write();

            if r && w {
                mio::Interest::READABLE | mio::Interest::WRITABLE
            } else if w {
                if self.state == State::Writing {
                    mio::Interest::WRITABLE
                } else {
                    mio::Interest::READABLE | mio::Interest::WRITABLE
                }
            } else {
                if self.state == State::Reading {
                    mio::Interest::READABLE
                } else {
                    mio::Interest::READABLE | mio::Interest::WRITABLE
                }
            }
        } else {
            match &self.state {
                State::Reading | State::Connected => Interest::READABLE,
                State::Writing => Interest::WRITABLE,
                State::Connecting => Interest::READABLE | Interest::WRITABLE,
            }
        }
    }

    pub fn register(&mut self, poll: &Poll) {
        let interests = self.interests();
        poll.registry()
            .register(&mut self.stream, self.token, interests)
            .unwrap();
    }

    pub fn reregister(&mut self, poll: &Poll) {
        let interests = self.interests();
        poll.registry()
            .reregister(&mut self.stream, self.token, interests)
            .unwrap();
    }

    pub fn deregister(&mut self, poll: &Poll) {
        poll.registry().deregister(&mut self.stream).unwrap();
    }
}
