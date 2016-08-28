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

extern crate shuteye;
extern crate tic;
extern crate tiny_http;

use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Error,
    Hit,
    Miss,
    Ok,
    Closed,
    Timeout,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::Ok => write!(f, "ok"),
            Status::Error => write!(f, "error"),
            Status::Hit => write!(f, "hit"),
            Status::Miss => write!(f, "miss"),
            Status::Closed => write!(f, "closed"),
            Status::Timeout => write!(f, "timeout"),
        }
    }
}

pub fn run(mut receiver: tic::Receiver<Status>, windows: usize, duration: usize) {

    let mut window = 0;
    let mut warmup = true;

    let mut total = 0;
    let mut ok = 0;
    let mut hit = 0;
    let mut miss = 0;
    let mut error = 0;
    let mut closed = 0;
    let mut timeout = 0;

    debug!("stats: collection ready");
    loop {
        receiver.run_once();
        let meters = receiver.clone_meters();

        let &new_ok = meters.get_count(&Status::Ok).unwrap_or(&0);
        let &new_hit = meters.get_count(&Status::Hit).unwrap_or(&0);
        let &new_miss = meters.get_count(&Status::Miss).unwrap_or(&0);
        let &new_error = meters.get_count(&Status::Error).unwrap_or(&0);
        let &new_closed = meters.get_count(&Status::Closed).unwrap_or(&0);
        let &new_timeout = meters.get_count(&Status::Timeout).unwrap_or(&0);
        let &new_total = meters.get_combined_count().unwrap_or(&0);

        if warmup {
            info!("-----");
            info!("Warmup complete");
            warmup = false;
        } else {
            let rate = (new_total - total) as f64 / duration as f64;

            let t = (new_total - total) as f64;
            let success_rate = if t > 0.0 {
                100.0 * (t - (new_error - error) as f64) / t
            } else {
                0.0
            };

            let &new_hit = meters.get_count(&Status::Hit).unwrap_or(&0);
            let &new_miss = meters.get_count(&Status::Miss).unwrap_or(&0);
            let t = ((new_hit - hit) + (new_miss - miss)) as f64;
            let hit_rate = if t > 0.0 {
                100.0 * (new_hit - hit) as f64 / t
            } else {
                0.0
            };

            info!("-----");
            info!("Window: {}", window);
            info!("Requests: Total: {} Timeout: {}",
                  (new_total - total),
                  (new_timeout - timeout));
            info!("Responses: Ok: {} Error: {} Closed: {} Hit: {} Miss: {}",
                  (new_ok - ok),
                  (new_error - error),
                  (new_closed - closed),
                  (new_hit - hit),
                  (new_miss - miss));
            info!("Rate: {:.*} rps Success: {:.*} % Hitrate: {:.*} %",
                  2,
                  rate,
                  2,
                  success_rate,
                  2,
                  hit_rate);
            info!("Latency: min: {} ns max: {} ns",
                    meters.get_combined_percentile(
                        tic::Percentile("min".to_owned(), 0.00)).unwrap_or(&0),
                    meters.get_combined_percentile(
                        tic::Percentile("max".to_owned(), 100.00)).unwrap_or(&0),
                );
            info!("Percentiles: p50: {} ns p90: {} ns p99: {} ns p999: {} ns p9999: {} ns",
                    meters.get_combined_percentile(
                        tic::Percentile("p50".to_owned(), 50.0)).unwrap_or(&0),
                    meters.get_combined_percentile(
                        tic::Percentile("p90".to_owned(), 90.0)).unwrap_or(&0),
                    meters.get_combined_percentile(
                        tic::Percentile("p99".to_owned(), 99.0)).unwrap_or(&0),
                    meters.get_combined_percentile(
                        tic::Percentile("p999".to_owned(), 99.9)).unwrap_or(&0),
                    meters.get_combined_percentile(
                        tic::Percentile("p9999".to_owned(), 99.99)).unwrap_or(&0),
                );
        }

        error = new_error;
        total = new_total;
        ok = new_ok;
        timeout = new_timeout;
        closed = new_closed;
        hit = new_hit;
        miss = new_miss;

        window += 1;

        if window > windows {
            receiver.save_trace();
            receiver.save_waterfall();
            break;
        }
    }
}
