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

pub const BUCKET_SIZE: usize = 10_000;

use cfgtypes;
use common::Queue;
use common::limits::Ratelimit;
use std::thread;

use cfgtypes::ProtocolGen;

/// Launch each of the workloads in their own thread
pub fn launch_workloads(workloads: Vec<cfgtypes::BenchmarkWorkload>,
                        work_queue: Queue<Vec<u8>>) {

    for (i, w) in workloads.into_iter().enumerate() {
        info!("Workload {}: Method: {} Rate: {}",
              i,
              w.gen.method(),
              w.rate);

        let mut workload = Workload::new(w.gen, Some(w.rate as u64), work_queue.clone()).unwrap();

        thread::spawn(move || {
            loop {
                workload.run();
            }
        });
    }
}

struct Workload {
    protocol: Box<ProtocolGen>,
    ratelimit: Option<Ratelimit>,
    queue: Queue<Vec<u8>>,
}

impl Workload {
    fn new(protocol: Box<ProtocolGen>,
           rate: Option<u64>,
           queue: Queue<Vec<u8>>)
           -> Result<Workload, &'static str> {
        let mut ratelimit = None;
        if let Some(r) = rate {
            if r != 0 {
                ratelimit = Some(Ratelimit::configure()
                    .frequency(r as u32)
                    .capacity(10000)
                    .build());
            }
        }
        Ok(Workload {
            protocol: protocol,
            ratelimit: ratelimit,
            queue: queue,
        })
    }

    fn run(&mut self) {
        loop {
            if let Some(ref mut ratelimit) = self.ratelimit {
                ratelimit.block(1);
            }

            let query = self.protocol.generate_message();
            let _ = self.queue.push(query);
        }
    }
}
