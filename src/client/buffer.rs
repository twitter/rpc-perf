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

use bytes::{ByteBuf, MutByteBuf};

#[derive(Debug)]
pub struct Buffer {
    rx_bytes: usize,
    tx_bytes: usize,
    pub rx: Option<MutByteBuf>,
    pub tx: Option<MutByteBuf>,
}

impl Buffer {
    pub fn new(rx: usize, tx: usize) -> Buffer {
        Buffer {
            rx_bytes: rx,
            tx_bytes: tx,
            rx: Some(ByteBuf::mut_with_capacity(rx)),
            tx: Some(ByteBuf::mut_with_capacity(tx)),
        }
    }

    pub fn clear(&mut self) {
        let mut rx = self.rx.take().unwrap_or_else(
            || ByteBuf::mut_with_capacity(self.rx_bytes),
        );
        rx.clear();
        self.rx = Some(rx);

        let mut tx = self.tx.take().unwrap_or_else(
            || ByteBuf::mut_with_capacity(self.tx_bytes),
        );
        tx.clear();
        self.tx = Some(tx);
    }
}
