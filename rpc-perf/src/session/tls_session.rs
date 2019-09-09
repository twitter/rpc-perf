// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use buffer::Buffer;
use rustls::Session as OtherSession;
use rustls::{ClientConfig, ClientSession};

use crate::session::*;

use std::{
    fmt::Display, io::Error, io::ErrorKind, io::Read, io::Write, net::ToSocketAddrs, sync::Arc,
};

pub struct TLSSession {
    common: Common,
    config: ClientConfig,
    stream: Stream,
    session: ClientSession,
    buffer: Buffer,
}

impl TLSSession {
    /// Create a new `TLSSession` which uses the configured `ClientSession` for TLS
    pub fn new<T: ToSocketAddrs + Display>(address: T, config: ClientConfig) -> Self {
        let hostname =
            webpki::DNSNameRef::try_from_ascii_str("localhost").expect("invalid dns name");
        let session = ClientSession::new(&Arc::new(config.clone()), hostname);
        Self {
            common: Common::new(),
            config,
            stream: Stream::new(address),
            session,
            buffer: Buffer::new(4096, 4096),
        }
    }
}

impl Session for TLSSession {
    fn stream(&self) -> &Stream {
        &self.stream
    }

    fn stream_mut(&mut self) -> &mut Stream {
        &mut self.stream
    }

    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    // TLSSession may need to read from the underlying stream to do session management
    fn session_read(&mut self) -> Result<(), Error> {
        if self.session.wants_read() {
            match self.session.read_tls(&mut self.stream) {
                Ok(0) => {
                    debug!("no bytes");
                    Ok(())
                }
                Err(e) => {
                    debug!("tls read error: {}", e);
                    Err(Error::new(ErrorKind::Other, "tls read error"))
                }
                _ => {
                    if self.session.process_new_packets().is_ok() {
                        Ok(())
                    } else {
                        error!("tls error processing packets");
                        Err(Error::new(ErrorKind::Other, "tls error"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    // TLSSession may need to write to the underlying stream to do session management
    fn session_flush(&mut self) -> Result<(), Error> {
        if self.session.wants_write() {
            if self.session.write_tls(&mut self.stream).is_err() {
                Err(Error::new(ErrorKind::Other, "tls write error"))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    // TLSSession does handshaking, status can be checked by calling function on underlying session
    fn is_handshaking(&self) -> bool {
        self.session.is_handshaking()
    }

    // clear the buffer
    fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    // create a new session
    fn session_reset(&mut self) {
        let hostname =
            webpki::DNSNameRef::try_from_ascii_str("localhost").expect("invalid dns name");
        self.session = ClientSession::new(&Arc::new(self.config.clone()), hostname);
    }

    fn read_buf(&self) -> &[u8] {
        self.buffer.rx_buffer()
    }

    fn write_buf(&mut self) -> &mut BytesMut {
        self.buffer.tx_buffer()
    }

    fn read_to(&mut self) -> Result<usize, Error> {
        self.session_read()?;
        match self.buffer.read_from(&mut self.session) {
            Ok(Some(0)) => {
                trace!("connection closed on read");
                Ok(0)
            }
            Ok(Some(n)) => {
                trace!("read {} bytes", n);
                Ok(n)
            }
            Ok(None) => {
                trace!("spurious read");
                Err(Error::new(ErrorKind::Other, "spurious read"))
            }
            Err(e) => {
                trace!("connection read error: {}", e);
                Err(e)
            }
        }
    }
}

impl Read for TLSSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.session_read()?;
        match self.buffer.read_from(&mut self.session) {
            Ok(Some(0)) => {
                trace!("connection closed on read");
                Ok(0)
            }
            Ok(Some(n)) => {
                trace!("read {} bytes", n);
                self.buffer.read(buf)
            }
            Ok(None) => {
                trace!("spurious read");
                Err(Error::new(ErrorKind::Other, "spurious read"))
            }
            Err(e) => {
                trace!("connection read error: {}", e);
                Err(e)
            }
        }
    }
}

impl Write for TLSSession {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        trace!("flush the connection");
        self.set_timestamp(Some(time::precise_time_ns()));

        let result = match self.buffer.write_to(&mut self.session) {
            Ok(Some(bytes)) => {
                // successful write
                trace!("flushed entire buffer {} bytes", bytes);
                Ok(())
            }
            Ok(None) => {
                // socket wasn't ready
                debug!("spurious call to flush");
                Err(Error::new(ErrorKind::Other, "spurious flush"))
            }
            Err(e) => {
                debug!("flush error: {:?}", e);
                Err(e)
            }
        };
        self.session_flush()?;
        result
    }
}
