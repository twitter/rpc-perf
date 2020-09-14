// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::session::*;

use buffer::Buffer;
use bytes::BytesMut;

use std::{fmt::Display, io::Error, io::ErrorKind, io::Read, io::Write, net::ToSocketAddrs};

/// Represents a plain `Session` over a `Stream`
pub struct PlainSession {
    common: Common,
    stream: Stream,
    buffer: Buffer,
}

impl PlainSession {
    /// Create a new `Session` which will operate a `Stream` to the given address
    pub fn new<T: ToSocketAddrs + Display>(address: T) -> Self {
        Self {
            common: Common::new(),
            stream: Stream::new(address),
            buffer: Buffer::new(4096, 4096),
        }
    }
}

impl Session for PlainSession {
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

    // plain sessions don't need any reads for session management
    fn session_read(&mut self) -> Result<(), Error> {
        Ok(())
    }

    // plain sessions don't need to flush any writes for session management
    fn session_flush(&mut self) -> Result<(), Error> {
        Ok(())
    }

    // plain sessions don't do any negotiation
    fn is_handshaking(&self) -> bool {
        false
    }

    // clear the buffer
    fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    // do nothing for plain session
    fn session_reset(&mut self) {}

    fn read_buf(&self) -> &[u8] {
        self.buffer.rx_buffer()
    }

    fn write_buf(&mut self) -> &mut BytesMut {
        self.buffer.tx_buffer()
    }

    fn read_to(&mut self) -> Result<usize, Error> {
        match self.buffer.read_from(&mut self.stream) {
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

impl Write for PlainSession {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        trace!("flush the connection");
        self.set_timestamp(Some(Instant::now()));

        match self.buffer.write_to(&mut self.stream) {
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
        }
    }
}

impl Read for PlainSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self.buffer.read_from(&mut self.stream) {
            Ok(Some(0)) => {
                trace!("connection closed on read");
                Ok(0)
            }
            Ok(Some(n)) => {
                trace!("read {} bytes", n);
                let _ = self.buffer.read(buf);
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
