// Connection

extern crate mio;

use std::net::{SocketAddr, ToSocketAddrs};

use net::InternetProtocol;
use std::io::{self, Read, Write};
use bytes::{Buf, MutBuf, ByteBuf, MutByteBuf};
use mio::timer::Timeout;
use mio::tcp::TcpStream;
use std::process::exit;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Closed,
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
            rx: Some(ByteBuf::mut_with_capacity(4 * 1024)),
            tx: Some(ByteBuf::mut_with_capacity(4 * 1024)),
        }
    }

    pub fn clear(&mut self) {
        let mut rx = self.rx.take().unwrap_or_else(|| ByteBuf::mut_with_capacity(4 * 1024));
        rx.clear();
        self.rx = Some(rx);

        let mut tx = self.tx.take().unwrap_or_else(|| ByteBuf::mut_with_capacity(4 * 1024));
        tx.clear();
        self.tx = Some(tx);
    }
}

#[derive(Debug)]
pub struct Connection {
    server: String,
    stream: Option<TcpStream>,
    state: State,
    buffer: Buffer,
    timeout: Option<Timeout>,
}

fn connect(server: &str, protocol: InternetProtocol) -> Result<TcpStream, &'static str> {
    if let Ok(mut a) = server.to_socket_addrs() {
        if let Ok(s) = to_mio_tcp_stream(a.next().unwrap(), protocol) {
            return Ok(s);
        }
        return Err("error connecting");
    }
    Err("error resolving")
}

fn to_mio_tcp_stream<T: ToSocketAddrs>(addr: T,
                                       proto: InternetProtocol)
                                       -> Result<TcpStream, &'static str> {
    match addr.to_socket_addrs() {
        Ok(r) => {
            for a in r {
                match a {
                    SocketAddr::V4(_) => {
                        if proto == InternetProtocol::Any || proto == InternetProtocol::IpV4 {
                            match TcpStream::connect(&a) {
                                Ok(s) => {
                                    return Ok(s);
                                }
                                Err(e) => {
                                    println!("some error: {}", e);
                                }
                            }
                        }
                    }
                    SocketAddr::V6(_) => {
                        if proto == InternetProtocol::Any || proto == InternetProtocol::IpV6 {
                            match TcpStream::connect(&a) {
                                Ok(s) => {
                                    return Ok(s);
                                }
                                Err(e) => {
                                    println!("some error: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err("Could not connect")
        }
        Err(_) => Err("Could not resolve"),
    }
}

impl Connection {
    /// create connection
    pub fn new(server: String) -> Connection {
        if let Ok(c) = connect(&server, InternetProtocol::Any) {
            Connection {
                server: server,
                stream: Some(c),
                state: State::Writing,
                buffer: Buffer::new(),
                timeout: None,
            }
        } else {
            Connection {
                server: server,
                stream: None,
                state: State::Closed,
                buffer: Buffer::new(),
                timeout: None,
            }
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
        if let Ok(s) = connect(&self.server, InternetProtocol::Any) {
            self.stream = Some(s);
            self.state = State::Writing;
        } else {
            error!("failed to reconnect");
        }
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
                    let _ = buffer.by_ref().take(n as u64).read_to_end(&mut response);
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

    pub fn event_set(&self) -> mio::Ready {
        match self.state {
            State::Reading => mio::Ready::readable(),
            State::Writing => mio::Ready::writable(),
            _ => mio::Ready::none() | mio::Ready::hup(),
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
