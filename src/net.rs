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

use mio::tcp::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum InternetProtocol {
    IpV4,
    IpV6,
    Any,
    None,
}

// custom Debug trait to show protocol name in human form
impl fmt::Debug for InternetProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InternetProtocol::IpV4 => write!(f, "IP::v4"),
            InternetProtocol::IpV6 => write!(f, "IP::v6"),
            InternetProtocol::Any => write!(f, "IP::Any"),
            InternetProtocol::None => write!(f, "IP::None"),
        }
    }
}

pub fn to_mio_tcp_stream<T: ToSocketAddrs>(addr: T,
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
