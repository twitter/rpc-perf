//  Copyright 2019 Twitter, Inc
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

mod client;
mod codec;
mod config;
mod session;
mod stats;

use crate::client::*;
use crate::codec::Codec;
use crate::config::Config;
use crate::config::Protocol;
use crate::stats::*;
use atomics::AtomicPrimitive;

use datastructures::AtomicBool;
pub(crate) use logger::*;
use metrics::Reading;
use ratelimiter::Ratelimiter;

use rand::thread_rng;

use std::sync::{atomic::Ordering, Arc, Mutex};
use std::thread;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let config = config::Config::new();

    Logger::new()
        .label("rpc_perf")
        .level(config.logging())
        .init()
        .expect("Failed to initialize logger");

    let metrics = Simple::new(&config);
    let recorder = metrics.recorder();

    stats::register_stats(&recorder);

    let mut stats_stdout = stats::StandardOut::new(metrics.recorder(), config.interval() as u64);

    let readings = Arc::new(Mutex::new(Vec::<Reading>::new()));
    if let Some(stats_listen) = config.listen() {
        let mut stats_http = stats::Http::new(stats_listen, metrics.recorder());
        let _ = thread::Builder::new()
            .name("http".to_string())
            .spawn(move || loop {
                stats_http.run();
            });
    }

    info!("rpc-perf {} initializing...", VERSION);

    config.print();

    do_warmup(&config, &metrics);

    let control = Arc::new(AtomicBool::new(true));
    launch_clients(&config, &metrics, control.clone());

    loop {
        std::thread::sleep(std::time::Duration::new(config.interval() as u64, 0));
        recorder.increment(Stat::Window);
        stats_stdout.print();

        if let Some(max_window) = config.windows() {
            if recorder.counter(Stat::Window) >= max_window as u64 {
                control.store(false, Ordering::SeqCst);
                break;
            }
        }
        let current = recorder.readings();
        let mut readings = readings.lock().unwrap();
        *readings = current;
        recorder.latch();
    }
    if let Some(waterfall) = config.waterfall() {
        recorder.save_waterfall(waterfall);
    }
}

fn do_warmup(config: &Config, metrics: &Simple) {
    if let Some(target) = config.warmup_hitrate() {
        info!("-----");
        info!("Warming the cache...");
        let recorder = metrics.recorder();
        let control = Arc::new(AtomicBool::new(true));
        launch_clients(&config, &metrics, control.clone());

        let mut warm = 0;
        loop {
            std::thread::sleep(std::time::Duration::new(config.interval() as u64, 0));
            recorder.increment(Stat::Window);

            let hit = recorder.counter(Stat::ResponsesHit) as f64;
            let miss = recorder.counter(Stat::ResponsesMiss) as f64;
            let hitrate = hit / (hit + miss);

            debug!("Hit-rate: {:.2}%", hitrate * 100.0);
            if hitrate >= target {
                warm += 1;
            } else {
                warm = 0;
            }

            if warm >= 3 {
                recorder.zero();
                control.store(false, Ordering::SeqCst);
                break;
            }

            recorder.zero();
        }

        info!("Warmup complete.");
    }
}

#[cfg(feature = "tls")]
fn make_client(id: usize, codec: Box<Codec>, config: &Config) -> Box<Client> {
    if config.tls_ca().is_some() && config.tls_key().is_some() && config.tls_cert().is_some() {
        let mut client = TLSClient::new(id, codec);
        if let Some(cafile) = config.tls_ca() {
            client.load_ca(&cafile);
        }

        if let Some(keyfile) = config.tls_key() {
            if let Some(certfile) = config.tls_cert() {
                client.load_key_and_cert(&keyfile, &certfile);
            }
        }
        Box::new(client)
    } else {
        Box::new(PlainClient::new(id, codec))
    }
}

#[cfg(not(feature = "tls"))]
fn make_client(id: usize, codec: Box<Codec>, _config: &Config) -> Box<Client> {
    Box::new(PlainClient::new(id, codec))
}

fn launch_clients(config: &Config, metrics: &stats::Simple, control: Arc<AtomicBool>) {
    let request_ratelimiter = if let Some(limit) = config.request_ratelimit() {
        Some(Arc::new(Ratelimiter::new(
            config.clients() as u64,
            1,
            limit as u64,
        )))
    } else {
        None
    };

    let connect_ratelimiter = if let Some(limit) = config.connect_ratelimit() {
        Some(Arc::new(Ratelimiter::new(
            config.clients() as u64,
            1,
            limit as u64,
        )))
    } else {
        None
    };

    let close_rate = if let Some(rate) = config.close_rate() {
        Some(Arc::new(Ratelimiter::new(
            config.clients() as u64,
            1,
            rate as u64,
        )))
    } else {
        None
    };

    for i in 0..config.clients() {
        let mut codec: Box<Codec> = match config.protocol() {
            Protocol::Echo => Box::new(crate::codec::Echo::new()),
            Protocol::Memcache => Box::new(crate::codec::Memcache::new()),
            Protocol::Ping => Box::new(crate::codec::Ping::new()),
            Protocol::RedisResp => {
                Box::new(crate::codec::Redis::new(crate::codec::RedisMode::Resp))
            }
            Protocol::RedisInline => {
                Box::new(crate::codec::Redis::new(crate::codec::RedisMode::Inline))
            }
        };

        // TODO: use a different generator for warmup
        codec.set_generator(config.generator());
        codec.set_recorder(metrics.recorder());

        let mut client = make_client(i, codec, config);
        client.set_poolsize(config.poolsize());
        client.set_tcp_nodelay(config.tcp_nodelay());
        client.set_close_rate(close_rate.clone());
        client.set_connect_ratelimit(connect_ratelimiter.clone());
        client.set_request_ratelimit(request_ratelimiter.clone());
        client.set_stats(metrics.recorder());
        client.set_connect_timeout(config.connect_timeout());
        client.set_request_timeout(config.request_timeout());

        let endpoints = config.endpoints();

        for endpoint in endpoints {
            client.add_endpoint(&endpoint);
        }

        let control = control.clone();
        let _ = thread::Builder::new()
            .name(format!("client{}", i).to_string())
            .spawn(move || {
                let mut rng = thread_rng();
                while control.load(Ordering::SeqCst) {
                    client.run(&mut rng);
                }
            });
    }
}
