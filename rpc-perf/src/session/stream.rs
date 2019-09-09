// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use mio::event::Evented;
use mio::net::TcpStream;
use mio::{Poll, PollOpt, Ready, Token};

use std::fmt::Display;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};

/// Holds the `Stream`'s address and underlying stream
pub struct Stream {
    address: SocketAddr,
    stream: Option<TcpStream>,
    nodelay: bool,
}

impl Stream {
    /// Create a new `Stream` which will be connected to the given address
    pub fn new<T: ToSocketAddrs + Display>(address: T) -> Self {
        let address = address
            .to_socket_addrs()
            .unwrap_or_else(|_| panic!("Failed to resole: {}", address))
            .next()
            .expect("No address");
        Self {
            address,
            stream: None,
            nodelay: false,
        }
    }

    /// Connects the underlying stream to the stored address
    pub fn connect(&mut self) -> Result<(), std::io::Error> {
        let stream = TcpStream::connect(&self.address)?;
        stream.set_nodelay(self.nodelay)?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Closes the underlying stream by dropping it
    pub fn close(&mut self) {
        self.stream = None;
    }

    /// Register the underlying stream with an event loop
    pub fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), Error> {
        if let Some(ref stream) = self.stream {
            stream.register(poll, token, interest, opts)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to register null TcpStream",
            ))
        }
    }

    /// Reregister the underlying stream with an event loop
    pub fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> Result<(), Error> {
        if let Some(ref stream) = self.stream {
            stream.reregister(poll, token, interest, opts)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to reregister null TcpStream",
            ))
        }
    }

    /// Deregister the underlying stream from the event loop
    pub fn deregister(&self, poll: &Poll) -> Result<(), Error> {
        if let Some(ref stream) = self.stream {
            stream.deregister(poll)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to deregister null TcpStream",
            ))
        }
    }

    /// Set the value for the NODELAY option
    pub fn set_nodelay(&mut self, nodelay: bool) -> Result<(), std::io::Error> {
        self.nodelay = nodelay;
        if let Some(stream) = &self.stream {
            stream.set_nodelay(nodelay)
        } else {
            Ok(())
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if let Some(ref mut stream) = self.stream {
            stream.read(buf)
        } else {
            Err(Error::new(ErrorKind::Other, "no TcpStream"))
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        if let Some(ref mut stream) = self.stream {
            stream.write(buf)
        } else {
            Err(Error::new(ErrorKind::Other, "no TcpStream"))
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
