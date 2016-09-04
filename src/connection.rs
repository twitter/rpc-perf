//  rpc-perf - RPC Performance Testing
//  Copyright 2015 Twitter, Inc
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

extern crate mio;
extern crate tic;

use std::io::{self, Read, Write};

use bytes::{Buf, MutBuf, ByteBuf, MutByteBuf};
use mio::deprecated::EventLoop;
use mio::tcp::TcpStream;
use mio::timer::Timeout;
use tic::{Clocksource, Sample};

use client::Client;
use state::State;
use stats::Status;
use cfgtypes::{ParsedResponse, ProtocolParse};

const MEGABYTE: usize = 1024 * 1024;

pub struct Connection {
    pub socket: TcpStream,
    pub token: Option<mio::Token>,
    pub state: State,
    pub server: String,
    buf: Option<ByteBuf>,
    mut_buf: Option<MutByteBuf>,
    pub t0: u64,
    pub stats: tic::Sender<Status>,
    clocksource: Clocksource,
    protocol: Box<ProtocolParse>,
    pub timeout: Option<Timeout>,
}

impl Connection {
    pub fn new(socket: TcpStream,
               server: String,
               token: Option<mio::Token>,
               stats: tic::Sender<Status>,
               clocksource: Clocksource,
               protocol: Box<ProtocolParse>,
               tcp_nodelay: bool)
               -> Connection {

        let _ = socket.set_nodelay(tcp_nodelay);

        Connection {
            socket: socket,
            server: server,
            token: token,
            state: State::Writing,
            buf: Some(ByteBuf::none()),
            mut_buf: Some(ByteBuf::mut_with_capacity(4 * MEGABYTE)),
            t0: clocksource.counter(),
            stats: stats,
            clocksource: clocksource,
            protocol: protocol,
            timeout: None,
        }
    }

    pub fn ready(&mut self,
                 event_loop: &mut EventLoop<Client>,
                 events: mio::Ready,
                 work: Option<Vec<u8>>) {

        trace!("    connection-state={:?}", self.state);

        match self.state {
            State::Reading => {
                assert!(events.is_readable(),
                        "unexpected events; events={:?}",
                        events);
                let now = self.clocksource.counter();
                let response = self.read(event_loop);
                match response {
                    ParsedResponse::Hit => {
                        let _ = self.stats.send(Sample::new(self.t0, now, Status::Hit));
                    }
                    ParsedResponse::Ok => {
                        let _ = self.stats.send(Sample::new(self.t0, now, Status::Ok));
                    }
                    ParsedResponse::Miss => {
                        let _ = self.stats.send(Sample::new(self.t0, now, Status::Miss));
                    }
                    ParsedResponse::Incomplete => {}
                    ParsedResponse::Unknown => {
                        let _ = self.stats.send(Sample::new(self.t0, now, Status::Closed));
                    }
                    _ => {
                        let _ = self.stats.send(Sample::new(self.t0, now, Status::Error));
                        debug!("unexpected response: {:?}", response);
                    }
                }
                if response != ParsedResponse::Incomplete {
                    if let Some(timeout) = self.timeout.clone() {
                        event_loop.clear_timeout(&timeout);
                        self.timeout = None;
                    }
                }
            }
            State::Writing => {
                assert!(events.is_writable(),
                        "unexpected events; events={:?}",
                        events);
                if let Some(w) = work {
                    let mut buf = match self.mut_buf.take() {
                        Some(b) => b,
                        None => {
                            panic!("no mut_buf to take");
                        }
                    };
                    buf.clear();
                    buf.write_slice(&*w);
                    self.buf = Some(buf.flip());
                    self.write(event_loop)
                } else {
                    panic!("no work");
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn read(&mut self, event_loop: &mut EventLoop<Client>) -> ParsedResponse {

        trace!("read()");

        // response unknown until parsed
        let mut resp = ParsedResponse::Unknown;

        let mut buf = match self.mut_buf.take() {
            Some(b) => b,
            None => {
                panic!("read() no mut_buf");
            }
        };

        match self.socket.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                trace!("read() closed");
                self.state = State::Closed;
            }
            Ok(Some(n)) => {
                unsafe {
                    buf.advance(n);
                }

                // read bytes from connection
                trace!("read() bytes {}", n);

                let mut buf = buf.flip();

                // protocol dependant parsing
                let mut response = Vec::<u8>::new();
                let _ = buf.by_ref().take(n as u64).read_to_end(&mut response);
                trace!("read: {:?}", response);
                resp = self.protocol.parse(&response);

                // if incomplete replace the buffer contents, otherwise transition
                match resp {
                    ParsedResponse::Incomplete => {
                        trace!("read() Incomplete");
                        self.mut_buf = Some(buf.resume());
                    }
                    _ => {
                        trace!("read() Complete");

                        self.state = State::Writing;
                        self.mut_buf = Some(buf.flip());
                    }
                }

                self.reregister(event_loop);
            }
            Ok(None) => {
                trace!("read() spurious wake-up");
                self.mut_buf = Some(buf);
                self.reregister(event_loop);
            }
            Err(e) => {
                debug!("server has terminated: {}", e);
                self.state = State::Closed;
            }
        }
        resp
    }

    pub fn write(&mut self, event_loop: &mut EventLoop<Client>) {
        trace!("write()");
        self.state = State::Writing;
        self.t0 = self.clocksource.counter();
        let mut buf = self.buf.take().unwrap();
        match self.socket.try_write_buf(&mut buf) {
            Ok(Some(_)) => {
                // successful write
                if !buf.has_remaining() {
                    self.state = State::Reading;
                    trace!("switch to read()");
                }
                self.reregister(event_loop);
            }
            Ok(None) => {
                // socket wasn't ready
                self.reregister(event_loop);
            }
            Err(e) => {
                // got some write error, abandon
                debug!("got an error trying to write; err={:?}", e);
                let t1 = self.clocksource.counter();
                let _ = self.stats.send(Sample::new(self.t0, t1, Status::Closed));
                self.state = State::Closed
            }
        }
        self.mut_buf = Some(buf.flip());
    }

    pub fn reregister(&self, event_loop: &mut EventLoop<Client>) {
        event_loop.reregister(&self.socket,
                        self.token.unwrap(),
                        event_set(self.state.clone()),
                        mio::PollOpt::edge())
            .unwrap();
    }
}

// State to mio EventSet mapping
fn event_set(state: State) -> mio::Ready {
    match state {
        State::Reading => mio::Ready::readable(),
        State::Writing => mio::Ready::writable(),
        _ => mio::Ready::none(),
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
