//  rpc-perf - RPC Performance Testing
//  Copyright 2017 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

const RX_BUFFER: usize = 4 * 1024;
const TX_BUFFER: usize = 4 * 1024;

use super::net::InternetProtocol;

use bytes::{Buf, MutBuf};
use client::buffer::Buffer;
use mio::Ready;
use mio::tcp::TcpStream;
use mio::unix::UnixReady;
use std::io::{self, Read, Write};
use std::net::SocketAddr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Closed,
    Connecting,
    Established,
    Reading,
    Writing,
}

pub struct Factory {
    rx: usize,
    tx: usize,
    connect_timeout: u64,
    max_connect_timeout: Option<u64>,
    request_timeout: u64,
    max_request_timeout: Option<u64>,
}

impl Factory {
    pub fn new(
        rx: usize,
        tx: usize,
        connect_timeout: u64,
        request_timeout: u64,
        max_connect_timeout: Option<u64>,
        max_request_timeout: Option<u64>,
    ) -> Factory {
        Factory {
            rx,
            tx,
            connect_timeout,
            request_timeout,
            max_connect_timeout,
            max_request_timeout,
        }
    }

    pub fn connect(&self, address: SocketAddr) -> Connection {
        Connection::new(
            address,
            self.rx,
            self.tx,
            self.connect_timeout,
            self.request_timeout,
            self.max_connect_timeout,
            self.max_request_timeout,
        )
    }
}

impl Default for Factory {
    fn default() -> Self {
        Factory {
            rx: RX_BUFFER,
            tx: TX_BUFFER,
            request_timeout: 0,
            connect_timeout: 0,
            max_connect_timeout: None,
            max_request_timeout: None,
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    state: State,
    buffer: Buffer,
    timeout: Option<u64>,
    protocol: InternetProtocol,
    connect_failures: u64,
    connect_timeout: u64,
    max_connect_timeout: Option<u64>,
    request_failures: u64,
    request_timeout: u64,
    max_request_timeout: Option<u64>,
}

impl Connection {
    /// create connection with specified buffer sizes
    pub fn new(
        address: SocketAddr,
        rx: usize,
        tx: usize,
        connect_timeout: u64,
        request_timeout: u64,
        max_connect_timeout: Option<u64>,
        max_request_timeout: Option<u64>,
    ) -> Self {
        let mut c = Connection {
            stream: None,
            state: State::Connecting,
            buffer: Buffer::new(rx, tx),
            timeout: None,
            protocol: InternetProtocol::Any,
            addr: address,
            connect_failures: 0,
            connect_timeout,
            max_connect_timeout,
            request_failures: 0,
            request_timeout,
            max_request_timeout,
        };
        c.reconnect();
        c
    }

    pub fn request_timeout(&self) -> u64 {
        let exponent = 1 << self.request_failures;
        let timeout = self.request_timeout.saturating_mul(exponent);
        if let Some(max_timeout) = self.max_request_timeout {
            if timeout > max_timeout {
                max_timeout
            } else {
                timeout
            }
        } else {
            timeout
        }
    }

    pub fn connect_timeout(&self) -> u64 {
        let exponent = 1 << self.connect_failures;
        let timeout = self.connect_timeout.saturating_mul(exponent);
        if let Some(max_timeout) = self.max_connect_timeout {
            if timeout > max_timeout {
                max_timeout
            } else {
                timeout
            }
        } else {
            timeout
        }
    }

    pub fn clear_failures(&mut self) {
        self.connect_failures = 0;
        self.request_failures = 0;
    }

    pub fn connect_failed(&mut self) {
        self.connect_failures += 1;
    }

    pub fn request_failed(&mut self) {
        self.request_failures += 1;
    }

    pub fn close(&mut self) -> Option<TcpStream> {
        self.state = State::Closed;
        self.buffer.clear();
        self.stream.take()
    }

    pub fn connect(&mut self) {
        self.state = State::Connecting;

        if let Ok(s) = TcpStream::connect(&self.addr) {
            self.stream = Some(s);
        } else {
            debug!("Error connecting: {}", self.addr);
        }
    }

    pub fn get_timeout(&mut self) -> Option<u64> {
        self.timeout
    }

    pub fn set_timeout(&mut self, timeout: Option<u64>) {
        self.timeout = timeout;
    }

    /// reconnect the connection in write mode
    pub fn reconnect(&mut self) {
        if self.state == State::Reading {
            self.request_failures += 1
        } else {
            self.request_failures = 0;
        }
        let _ = self.close();
        self.connect();
    }

    pub fn stream(&self) -> Option<&TcpStream> {
        if let Some(ref s) = self.stream {
            Some(s)
        } else {
            None
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// flush the buffer
    pub fn flush(&mut self) -> Result<(), io::Error> {
        if self.state != State::Writing {
            error!("{:?} invalid for read", self.state);
            return Err(io::Error::new(io::ErrorKind::Other, "invalid state"));
        }
        let b = self.buffer.tx.take();
        if let Some(buffer) = b {
            let mut buffer = buffer.flip();
            let buffer_bytes = buffer.remaining();

            let mut s = self.stream.take().unwrap();

            match s.try_write_buf(&mut buffer) {
                Ok(Some(bytes)) => {
                    // successful write
                    trace!("flush {} out of {} bytes", bytes, buffer_bytes);
                    if !buffer.has_remaining() {
                        // write is complete
                        self.set_state(State::Reading);
                    } else {
                        // write is not complete
                        debug!("connection buffer not flushed completely")
                    }
                    self.stream = Some(s);
                }
                Ok(None) => {
                    // socket wasn't ready
                    self.stream = Some(s);
                    debug!("spurious call to flush flush");
                }
                Err(e) => {
                    // got some write error, abandon
                    debug!("flush error: {:?}", e);
                    return Err(e);
                }
            }
            self.buffer.tx = Some(buffer.flip());
            Ok(())
        } else {
            debug!("connection missing buffer on flush");
            return Err(io::Error::new(io::ErrorKind::Other, "buffer missing"));
        }
    }

    /// write bytes into the buffer and call flush
    pub fn write(&mut self, bytes: Vec<u8>) -> Result<(), io::Error> {
        if self.state != State::Writing {
            error!("{:?} invalid for read", self.state);
            return Err(io::Error::new(io::ErrorKind::Other, "invalid state"));
        }
        trace!("write {} bytes", bytes.len());
        let b = self.buffer.tx.take();
        if let Some(mut buffer) = b {
            buffer.clear();
            buffer.write_slice(&bytes);
            self.buffer.tx = Some(buffer);
        } else {
            error!("connection missing buffer on write");
            return Err(io::Error::new(io::ErrorKind::Other, "buffer missing"));
        }
        self.flush()
    }

    pub fn set_state(&mut self, state: State) {
        trace!("connection state change {:?} to {:?}", self.state, state);
        self.state = state;
    }

    pub fn is_connecting(&self) -> bool {
        self.state == State::Connecting
    }

    pub fn is_readable(&self) -> bool {
        self.state == State::Reading
    }

    pub fn is_writable(&self) -> bool {
        self.state == State::Writing
    }

    pub fn read(&mut self) -> Result<Vec<u8>, io::Error> {
        if self.state() != State::Reading {
            error!("{:?} invalid for read", self.state);
            return Err(io::Error::new(io::ErrorKind::Other, "invalid state"));
        }

        let mut response = Vec::<u8>::new();

        if let Some(mut buffer) = self.buffer.rx.take() {
            let mut s = self.stream.take().unwrap();
            match s.try_read_buf(&mut buffer) {
                Ok(Some(0)) => {
                    trace!("connection closed on read");
                    return Err(io::Error::new(io::ErrorKind::Other, "connection closed"));
                }
                Ok(Some(n)) => {
                    unsafe {
                        buffer.advance(n);
                    }

                    // read bytes from connection
                    trace!("read {} bytes", n);
                    let mut buffer = buffer.flip();
                    let _ = buffer.by_ref().take(n as u64).read_to_end(&mut response);
                    self.buffer.rx = Some(buffer.flip());
                    self.stream = Some(s);
                }
                Ok(None) => {
                    trace!("spurious read");
                    self.buffer.rx = Some(buffer);
                    self.stream = Some(s);
                }
                Err(e) => {
                    trace!("connection read error: {}", e);
                    return Err(e);
                }
            }
        } else {
            error!("connection missing buffer on read");
            return Err(io::Error::new(io::ErrorKind::Other, "missing buffer"));
        }
        Ok(response)
    }

    pub fn event_set(&self) -> Ready {
        match self.state {
            State::Connecting | State::Established | State::Writing => {
                Ready::writable() | UnixReady::hup()
            }
            State::Reading => Ready::readable() | UnixReady::hup(),
            _ => Ready::empty(),
        }
    }
}

pub trait TryRead {
    fn try_read_buf<B: MutBuf>(&mut self, buf: &mut B) -> io::Result<Option<usize>>
    where
        Self: Sized,
    {
        // Reads the length of the slice supplied by buf.mut_bytes into the buffer
        // This is not guaranteed to consume an entire datagram or segment.
        // If your protocol is msg based (instead of continuous stream) you should
        // ensure that your buffer is large enough to hold an entire segment
        // (1532 bytes if not jumbo frames)
        let res = self.try_read(unsafe { buf.mut_bytes() });

        if let Ok(Some(cnt)) = res {
            unsafe {
                buf.advance(cnt);
            }
        }

        res
    }

    fn try_read(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>>;
}

pub trait TryWrite {
    fn try_write_buf<B: Buf>(&mut self, buf: &mut B) -> io::Result<Option<usize>>
    where
        Self: Sized,
    {
        let res = self.try_write(buf.bytes());

        if let Ok(Some(cnt)) = res {
            buf.advance(cnt);
        }

        res
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<Option<usize>>;
}

impl<T: Read> TryRead for T {
    fn try_read(&mut self, dst: &mut [u8]) -> io::Result<Option<usize>> {
        self.read(dst).map_non_block()
    }
}

impl<T: Write> TryWrite for T {
    fn try_write(&mut self, src: &[u8]) -> io::Result<Option<usize>> {
        self.write(src).map_non_block()
    }
}

/// A helper trait to provide the `map_non_block` function on Results.
trait MapNonBlock<T> {
    /// Maps a `Result<T>` to a `Result<Option<T>>` by converting
    /// operation-would-block errors into `Ok(None)`.
    fn map_non_block(self) -> io::Result<Option<T>>;
}

impl<T> MapNonBlock<T> for io::Result<T> {
    fn map_non_block(self) -> io::Result<Option<T>> {
        use std::io::ErrorKind::WouldBlock;

        match self {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                if let WouldBlock = err.kind() {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}
