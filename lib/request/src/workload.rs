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

use cfgtypes::ProtocolGen;
use common::async::channel::{SyncSender, TrySendError};
use common::limits::Ratelimit;
use common::stats::{Clocksource, Sample, Sender, Stat};
use std::thread;

/// Launch each of the workloads in their own thread
pub fn launch_workloads(workloads: Vec<cfgtypes::BenchmarkWorkload>,
                        work_queue: Vec<SyncSender<Vec<u8>>>,
                        stats: Sender<Stat>,
                        clocksource: Clocksource) {

    for (i, w) in workloads.into_iter().enumerate() {
        info!("Workload {}: Method: {} Rate: {}",
              i,
              w.gen.method(),
              w.rate);

        let mut workload = Workload::new(w.gen,
                                         Some(w.rate as u64),
                                         work_queue.clone(),
                                         stats.clone(),
                                         clocksource.clone())
                .unwrap();

        let _ = thread::Builder::new()
            .name(format!("workload{}", i).to_string())
            .spawn(move || loop {
                       workload.run();
                   });
    }
}

struct Workload {
    protocol: Box<ProtocolGen>,
    ratelimit: Option<Ratelimit>,
    queue: Vec<SyncSender<Vec<u8>>>,
    stats: Sender<Stat>,
    clocksource: Clocksource,
}

impl Workload {
    /// Create a new `Workload` based on a protocol, optional rate, and a queue
    fn new(protocol: Box<ProtocolGen>,
           rate: Option<u64>,
           queue: Vec<SyncSender<Vec<u8>>>,
           stats: Sender<Stat>,
           clocksource: Clocksource)
           -> Result<Workload, &'static str> {
        let mut ratelimit = None;
        if let Some(r) = rate {
            if r > 0 {
                ratelimit = Some(Ratelimit::configure()
                                     .frequency(r as u32)
                                     .capacity(BUCKET_SIZE as u32)
                                     .build());
            }
        }
        Ok(Workload {
               protocol: protocol,
               ratelimit: ratelimit,
               queue: queue,
               stats: stats,
               clocksource: clocksource,
           })
    }

    /// Generates work at a fixed rate and pushes to the queue
    fn run(&mut self) {
        let mut index = 0;
        loop {
            if let Some(ref mut ratelimit) = self.ratelimit {
                ratelimit.block(1);
            }
            let t0 = self.clocksource.counter();
            let mut msg = Some(self.protocol.generate_message());
            loop {
                match self.queue[index].try_send(msg.take().unwrap()) {
                    Ok(_) => {
                        let t1 = self.clocksource.counter();
                        let _ = self.stats
                            .send(Sample::new(t0, t1, Stat::RequestPrepared));
                        break;
                    }
                    Err(e) => {
                        match e {
                            TrySendError::Full(m) => {
                                msg = Some(m);
                            }
                            _ => {
                                error!("Receiving thread died?");
                            }
                        }
                    }
                }
                index += 1;
                if index >= self.queue.len() {
                    index = 0;
                }
            }
        }
    }
}
