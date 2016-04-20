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

const ONE_SECOND: u64 = 1_000_000_000;
pub const BUCKET_SIZE: usize = 10_000;

use cfgtypes;
use mpmc;
use ratelimit::Ratelimit;
use std::thread;
use time;

use cfgtypes::ProtocolGen;

/// Launch each of the workloads in their own thread
pub fn launch_workloads(workloads: Vec<cfgtypes::BenchmarkWorkload>,
                    work_queue: mpmc::Queue<Vec<u8>>) {

    for (i, w) in workloads.into_iter().enumerate() {
        info!("Workload {}: Method: {} Rate: {}", i, w.gen.method(), w.rate);

        let mut workload = Workload::new(w.gen,
                                         Some(w.rate as u64),
                                         work_queue.clone())
                               .unwrap();

        thread::spawn(move || {
            loop {
                workload.run();
            }
        });
    }
}

struct Workload {
    protocol: Box<ProtocolGen>,
    rate: u64,
    ratelimit: Ratelimit,
    queue: mpmc::Queue<Vec<u8>>,
}

impl Workload {
    fn new(protocol: Box<ProtocolGen>,
               rate: Option<u64>,
               queue: mpmc::Queue<Vec<u8>>)
               -> Result<Workload, &'static str> {
        let r = rate.unwrap_or(0);
        let i = rate_to_interval(r);
        let ratelimit = match Ratelimit::new(BUCKET_SIZE, time::precise_time_ns(), i, 1) {
            Some(r) => r,
            None => {
                return Err("Ratelimit initialization failed!");
            }
        };
        Ok(Workload {
            protocol: protocol,
            rate: rate.unwrap_or(0),
            ratelimit: ratelimit,
            queue: queue
        })
    }

    fn run(&mut self) {
        loop {
            if self.rate != 0 {
                self.ratelimit.block(1);
            }

            let query = self.protocol.generate_message();
            let _ = self.queue.push(query);
        }
    }
}

fn rate_to_interval(rate: u64) -> u64 {
    if rate == 0 {
        return 0;
    }
    let interval = ONE_SECOND / rate;
    if interval < 1 {
        return 0;
    }
    interval
}
