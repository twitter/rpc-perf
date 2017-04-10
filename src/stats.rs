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

use request::BenchmarkConfig;
use common::stats::{Interest, Meters, Percentile, Receiver, Sample, Stat};
use std::process::exit;

pub fn stats_receiver_init(config: &BenchmarkConfig,
                       listen: Option<String>,
                       waterfall: Option<String>,
                       trace: Option<String>)
                       -> Receiver<Stat> {
    let mut stats_config = Receiver::<Stat>::configure()
        .batch_size(16)
        .capacity(65536)
        .duration(config.duration)
        .windows(config.windows);

    if let Some(addr) = listen {
        stats_config = stats_config.http_listen(addr);
    }

    let mut stats_receiver = stats_config.build();

    let counts = vec![Stat::Window,
                      Stat::ResponseOk,
                      Stat::ResponseOkHit,
                      Stat::ResponseOkMiss,
                      Stat::ResponseError,
                      Stat::ResponseTimeout,
                      Stat::RequestPrepared,
                      Stat::RequestSent,
                      Stat::ConnectOk,
                      Stat::ConnectError,
                      Stat::ConnectTimeout,
                      Stat::SocketCreate,
                      Stat::SocketClose,
                      Stat::SocketRead,
                      Stat::SocketFlush,
                      Stat::SocketWrite];

    for c in counts {
        stats_receiver.add_interest(Interest::Count(c));
    }

    for c in vec![Stat::ResponseOk, Stat::ResponseOkHit, Stat::ResponseOkMiss, Stat::ConnectOk] {
        stats_receiver.add_interest(Interest::Percentile(c));
    }

    if let Some(w) = waterfall {
        stats_receiver.add_interest(Interest::Waterfall(Stat::ResponseOk, w));
    }

    if let Some(t) = trace {
        stats_receiver.add_interest(Interest::Waterfall(Stat::ResponseOk, t));
    }

    stats_receiver
}


pub fn meters_delta(t0: &Meters<Stat>, t1: &Meters<Stat>, stat: &Stat) -> u64 {
    *t1.get_count(stat).unwrap_or(&0) - *t0.get_count(stat).unwrap_or(&0)
}

pub fn run(mut receiver: Receiver<Stat>, windows: usize, infinite: bool) {

    let mut window = 0;
    let mut warmup = true;

    let clocksource = receiver.get_clocksource().clone();
    let mut sender = receiver.get_sender().clone();
    sender.set_batch_size(1);

    let mut t0 = clocksource.counter();
    let mut m0 = receiver.clone_meters();

    debug!("stats: collection ready");
    loop {
        receiver.run_once();
        let t1 = clocksource.counter();
        let _ = sender.send(Sample::new(t0, t1, Stat::Window));
        let m1 = receiver.clone_meters();

        if warmup {
            info!("-----");
            if meters_delta(&m0, &m1, &Stat::ConnectOk) == 0 {
                error!("No connections established. Please check that server(s) are available");
                exit(1);
            }
            info!("Warmup complete");
            warmup = false;
        } else {
            let responses = meters_delta(&m0, &m1, &Stat::ResponseOk) +
                            meters_delta(&m0, &m1, &Stat::ResponseError);

            let rate = responses as f64 /
                       ((clocksource.convert(t1) - clocksource.convert(t0)) as f64 /
                        1_000_000_000.0);

            let success_rate = if responses > 0 {
                100.0 * (responses - meters_delta(&m0, &m1, &Stat::ResponseError)) as f64 /
                (responses + meters_delta(&m0, &m1, &Stat::ResponseTimeout)) as f64
            } else {
                0.0
            };

            let hit = meters_delta(&m0, &m1, &Stat::ResponseOkHit);
            let miss = meters_delta(&m0, &m1, &Stat::ResponseOkMiss);

            let hit_rate = if (hit + miss) > 0 {
                100.0 * hit as f64 / (hit + miss) as f64
            } else {
                0.0
            };

            info!("-----");
            info!("Window: {}", *m1.get_count(&Stat::Window).unwrap_or(&0));
            let inflight = *m1.get_count(&Stat::RequestSent).unwrap_or(&0) as i64 -
                           *m1.get_count(&Stat::ResponseOk).unwrap_or(&0) as i64 -
                           *m1.get_count(&Stat::ResponseError).unwrap_or(&0) as i64 -
                           *m1.get_count(&Stat::ResponseTimeout).unwrap_or(&0) as i64;
            let open = *m1.get_count(&Stat::SocketCreate).unwrap_or(&0) as i64 -
                       *m1.get_count(&Stat::SocketClose).unwrap_or(&0) as i64;
            info!("Connections: Ok: {} Error: {} Timeout: {} Open: {}",
                  meters_delta(&m0, &m1, &Stat::ConnectOk),
                  meters_delta(&m0, &m1, &Stat::ConnectError),
                  meters_delta(&m0, &m1, &Stat::ConnectTimeout),
                  open);
            info!("Sockets: Create: {} Close: {} Read: {} Write: {} Flush: {}",
                  meters_delta(&m0, &m1, &Stat::SocketCreate),
                  meters_delta(&m0, &m1, &Stat::SocketClose),
                  meters_delta(&m0, &m1, &Stat::SocketRead),
                  meters_delta(&m0, &m1, &Stat::SocketWrite),
                  meters_delta(&m0, &m1, &Stat::SocketFlush));
            info!("Requests: Sent: {} Prepared: {} In-Flight: {}",
                  meters_delta(&m0, &m1, &Stat::RequestSent),
                  meters_delta(&m0, &m1, &Stat::RequestPrepared),
                  inflight);
            info!("Responses: Ok: {} Error: {} Timeout: {} Hit: {} Miss: {}",
                  meters_delta(&m0, &m1, &Stat::ResponseOk),
                  meters_delta(&m0, &m1, &Stat::ResponseError),
                  meters_delta(&m0, &m1, &Stat::ResponseTimeout),
                  meters_delta(&m0, &m1, &Stat::ResponseOkHit),
                  meters_delta(&m0, &m1, &Stat::ResponseOkMiss));
            info!("Rate: {:.*} rps Success: {:.*} % Hit Rate: {:.*} %",
                  2,
                  rate,
                  2,
                  success_rate,
                  2,
                  hit_rate);
            display_percentiles(&m1, &Stat::ResponseOk, "Response OK");
        }

        m0 = m1;
        t0 = t1;
        window += 1;

        if window > windows {
            receiver.save_files();
            if infinite {
                window = 0;
                receiver.clear_heatmaps();
            } else {
                break;
            }
        }
    }
}

fn display_percentiles(meters: &Meters<Stat>, stat: &Stat, label: &str) {
    info!("Percentiles: {} (us): min: {} p50: {} p90: {} p99: {} p999: {} p9999: {} max: {}",
                    label,
                    meters.get_percentile(stat,
                        Percentile("min".to_owned(), 0.0)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("p50".to_owned(), 50.0)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("p90".to_owned(), 90.0)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("p99".to_owned(), 99.0)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("p999".to_owned(), 99.9)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("p9999".to_owned(), 99.99)).unwrap_or(&0) / 1000,
                    meters.get_percentile(stat,
                        Percentile("max".to_owned(), 100.0)).unwrap_or(&0) / 1000,
                );
}
