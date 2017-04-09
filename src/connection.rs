// Connection


use bytes::{Buf, ByteBuf, MutBuf, MutByteBuf};

use common;
use common::async::tcp::TcpStream;
use common::async::timer::Timeout;
use net::InternetProtocol;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::process::exit;

const RX_BUFFER: usize = 4 * 1024;
const TX_BUFFER: usize = 4 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Closed,
    Connecting,
    Reading,
    Writing,
}

#[derive(Debug)]
struct Buffer {
    rx: Option<MutByteBuf>,
    tx: Option<MutByteBuf>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            rx: Some(ByteBuf::mut_with_capacity(RX_BUFFER)),
            tx: Some(ByteBuf::mut_with_capacity(TX_BUFFER)),
        }
    }

    pub fn clear(&mut self) {
        let mut rx = self.rx
            .take()
            .unwrap_or_else(|| ByteBuf::mut_with_capacity(RX_BUFFER));
        rx.clear();
        self.rx = Some(rx);

        let mut tx = self.tx
            .take()
            .unwrap_or_else(|| ByteBuf::mut_with_capacity(TX_BUFFER));
        tx.clear();
        self.tx = Some(tx);
    }
}

#[derive(Debug)]
pub struct Connection {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    state: State,
    buffer: Buffer,
    timeout: Option<Timeout>,
    protocol: InternetProtocol,
}

impl Connection {
    /// create connection
    pub fn new(address: SocketAddr) -> Connection {
        let mut c = Connection {
            stream: None,
            state: State::Connecting,
            buffer: Buffer::new(),
            timeout: None,
            protocol: InternetProtocol::Any,
            addr: address,
        };
        c.reconnect();
        c
    }

    pub fn connect(&mut self) {
        self.state = State::Connecting;

        if let Ok(s) = TcpStream::connect(&self.addr) {
            self.stream = Some(s);
        } else {
            debug!("Error connecting: {}", self.addr);
        }
    }

    pub fn get_timeout(&mut self) -> Option<Timeout> {
        self.timeout.take()
    }

    pub fn set_timeout(&mut self, timeout: Timeout) {
        self.timeout = Some(timeout);
    }

    /// reconnect the connection in write mode
    pub fn reconnect(&mut self) {
        let _ = self.close();
        self.connect();
    }

    pub fn close(&mut self) -> Option<TcpStream> {
        self.state = State::Closed;
        self.buffer.clear();
        self.stream.take()
    }

    pub fn stream(&self) -> Option<&TcpStream> {
        if let Some(ref s) = self.stream {
            Some(s)
        } else {
            None
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// flush the buffer
    pub fn flush(&mut self) -> Result<(), ()> {
        if self.state != State::Writing {
            error!("flush() {:?} connection", self.state);
            exit(1);
        }
        let b = self.buffer.tx.take();
        if let Some(buffer) = b {
            let mut buffer = buffer.flip();

            let mut s = self.stream.take().unwrap();

            match s.try_write_buf(&mut buffer) {
                Ok(Some(_)) => {
                    // successful write
                    if !buffer.has_remaining() {
                        self.set_readable();
                    } else {
                        debug!("incomplete write to skbuff")
                    }
                    self.stream = Some(s);
                }
                Ok(None) => {
                    // socket wasn't ready
                    self.stream = Some(s);
                    debug!("spurious read");
                }
                Err(e) => {
                    // got some write error, abandon
                    debug!("write error: {:?}", e);
                    return Err(());
                }
            }
            self.buffer.tx = Some(buffer.flip());
            Ok(())
        } else {
            error!("read() no buffer");
            Err(())
        }
    }

    pub fn write(&mut self, bytes: Vec<u8>) -> Result<(), ()> {
        if !self.is_writable() {
            error!("write() {:?} connection", self.state);
            exit(1);
        }
        trace!("write(): {:?}", bytes);
        let b = self.buffer.tx.take();
        if let Some(mut buffer) = b {
            buffer.clear();
            buffer.write_slice(&bytes);
            self.buffer.tx = Some(buffer);
        } else {
            error!("buffer error");
            exit(1);
        }
        self.flush()
    }

    pub fn set_writable(&mut self) {
        trace!("connection switch to writable");
        self.state = State::Writing;
    }

    pub fn set_readable(&mut self) {
        trace!("connection switch to readable");
        self.state = State::Reading;
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

    pub fn read(&mut self) -> Result<Vec<u8>, ()> {
        if !self.is_readable() {
            error!("read() {:?} connection", self.state);
            exit(1);
        }

        trace!("read()");

        let mut response = Vec::<u8>::new();

        if let Some(mut buffer) = self.buffer.rx.take() {
            let mut s = self.stream.take().unwrap();
            match s.try_read_buf(&mut buffer) {
                Ok(Some(0)) => {
                    error!("read() closed");
                    return Err(());
                }
                Ok(Some(n)) => {
                    unsafe {
                        buffer.advance(n);
                    }

                    // read bytes from connection
                    trace!("read() bytes {}", n);
                    let mut buffer = buffer.flip();
                    let _ = buffer
                        .by_ref()
                        .take(n as u64)
                        .read_to_end(&mut response);
                    trace!("read: {:?}", response);
                    self.buffer.rx = Some(buffer.flip());
                    self.stream = Some(s);
                }
                Ok(None) => {
                    error!("read() spurious wake-up");
                    self.buffer.rx = Some(buffer);
                    self.stream = Some(s);
                }
                Err(e) => {
                    error!("read() server has terminated: {}", e);
                    return Err(());
                }
            }
        } else {
            error!("read() buffer issue");
            exit(1);
        }
        Ok(response)
    }

    pub fn event_set(&self) -> common::async::Ready {
        match self.state {
            State::Connecting | State::Writing => common::async::Ready::writable(),
            State::Reading => common::async::Ready::readable(),
            _ => common::async::Ready::none(),
        }
    }
}

pub trait TryRead {
    fn try_read_buf<B: MutBuf>(&mut self, buf: &mut B) -> io::Result<Option<usize>>
        where Self: Sized
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
        where Self: Sized
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
