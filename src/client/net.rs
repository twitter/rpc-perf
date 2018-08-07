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

use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum InternetProtocol {
    IpV4,
    IpV6,
    Any,
}

// custom Debug trait to show protocol name in human form
impl fmt::Debug for InternetProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InternetProtocol::IpV4 => write!(f, "IPv4"),
            InternetProtocol::IpV6 => write!(f, "IPv6"),
            InternetProtocol::Any => write!(f, "IP"),
        }
    }
}

pub fn choose_layer_3(ipv4: bool, ipv6: bool) -> Result<InternetProtocol, String> {
    if ipv4 && ipv6 {
        return Err("Use only --ipv4 or --ipv6".to_owned());
    }

    if !ipv4 && !ipv6 {
        return Ok(InternetProtocol::Any);
    } else if ipv4 {
        return Ok(InternetProtocol::IpV4);
    } else if ipv6 {
        return Ok(InternetProtocol::IpV6);
    }

    Err(
        "No InternetProtocols remaining! Bad config/options".to_owned(),
    )
}
