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

mod general;

pub use self::general::Protocol;

use crate::config::general::General;
use crate::*;

use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;

use clap::{App, Arg, ArgMatches};
use rand::distributions::{Alphanumeric, Distribution, Uniform};
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde_derive::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    general: General,
    keyspace: Vec<Keyspace>,
}

impl Default for Config {
    fn default() -> Config {
        let mut keyspace = Vec::new();
        let get = Command {
            action: Action::Get,
            weight: 1,
        };
        let set = Command {
            action: Action::Set,
            weight: 1,
        };
        let value = Value {
            length: 64,
            weight: 1,
        };
        keyspace.push(Keyspace {
            length: 8,
            count: Some(10_000_000),
            weight: 1,
            hitrate: None,
            commands: vec![get, set],
            values: vec![value],
        });
        Config {
            general: Default::default(),
            keyspace,
        }
    }
}

#[derive(Copy, Clone, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Get,
    Set,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Keyspace {
    length: usize,
    weight: usize,
    count: Option<usize>,
    hitrate: Option<f64>,
    commands: Vec<Command>,
    values: Vec<Value>,
}

pub struct Generator {
    keyspaces: Vec<KeyspaceGenerator>,
}

impl Generator {
    pub fn generate(&self, rng: &mut ThreadRng) -> crate::codec::Command {
        let keyspace = self
            .keyspaces
            .choose_weighted(rng, |keyspace| keyspace.weight())
            .unwrap();
        let action = keyspace.choose_action(rng);
        match action {
            Action::Get => {
                let key = keyspace.choose_key(rng);
                crate::codec::Command::get(key)
            }
            Action::Set => {
                let key = keyspace.choose_key(rng);
                let value = keyspace.choose_value(rng);
                crate::codec::Command::set(key, value)
            }
        }
    }
}

pub struct KeyspaceGenerator {
    length: usize,
    weight: usize,
    distribution: Uniform<usize>,
    commands: Vec<Command>,
    values: Vec<Value>,
}

impl KeyspaceGenerator {
    pub fn weight(&self) -> usize {
        self.weight
    }

    pub fn choose_action(&self, rng: &mut ThreadRng) -> Action {
        self.commands
            .choose_weighted(rng, |command| command.weight())
            .unwrap()
            .action()
    }

    pub fn choose_key(&self, rng: &mut ThreadRng) -> String {
        format!(
            "{:0width$}",
            self.distribution.sample(rng),
            width = self.length
        )
    }

    pub fn choose_value(&self, rng: &mut ThreadRng) -> String {
        let length = self
            .values
            .choose_weighted(rng, |value| value.weight())
            .unwrap()
            .length();
        rng.sample_iter(&Alphanumeric)
            .take(length)
            .collect::<String>()
    }
}

impl Keyspace {
    pub fn generator(&self) -> KeyspaceGenerator {
        let count = if let Some(count) = self.count {
            let digits = (count as f64).log10().ceil() as usize;
            if digits > self.length {
                fatal!(
                    "Keyspace with length: {} has count ({}) that can't be represented within key length",
                    self.length,
                    count,
                );
            }
            count
        } else {
            if self.length > (usize::max_value() as f64).log10().floor() as usize {
                fatal!(
                    "Keyspace with length: {} cannot be represented with usize",
                    self.length
                );
            }
            10_usize.pow(self.length as u32)
        };

        let distribution = Uniform::from(0..count);
        KeyspaceGenerator {
            length: self.length,
            weight: self.weight,
            distribution,
            commands: self.commands.clone(),
            values: self.values.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Command {
    action: Action,
    weight: usize,
}

impl Command {
    pub fn action(&self) -> Action {
        self.action
    }

    pub fn weight(&self) -> usize {
        self.weight
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Value {
    length: usize,
    weight: usize,
}

impl Value {
    pub fn length(&self) -> usize {
        self.length
    }

    pub fn weight(&self) -> usize {
        self.weight
    }
}

impl Config {
    /// parse command line options and return `Config`
    pub fn new() -> Config {
        let app = App::new(NAME)
            .version(VERSION)
            .author("Brian Martin <bmartin@twitter.com>")
            .about("RPC Performance Testing")
            .arg(
                Arg::with_name("config")
                    .long("config")
                    .value_name("FILE")
                    .help("TOML config file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("listen")
                    .long("listen")
                    .value_name("IP:PORT")
                    .help("Optional listen address for stats")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .long("verbose")
                    .help("Increase verbosity by one level. Can be used more than once")
                    .multiple(true),
            )
            .arg(
                Arg::with_name("interval")
                    .long("interval")
                    .value_name("Seconds")
                    .help("Integration window duration and period for stats output")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("windows")
                    .long("windows")
                    .value_name("Count")
                    .help("The number of intervals before exit")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("clients")
                    .long("clients")
                    .value_name("# Clients")
                    .help("The number of client threads / event-loops to run")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("poolsize")
                    .long("poolsize")
                    .value_name("# Connections")
                    .help("The number of connections from each client to each endpoint")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("service")
                    .long("service")
                    .help("Enable service-mode with unlimited windows"),
            )
            .arg(
                Arg::with_name("endpoint")
                    .long("endpoint")
                    .value_name("HOST:PORT or IP:PORT")
                    .help("Provide a server endpoint to test")
                    .multiple(true)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("protocol")
                    .long("protocol")
                    .value_name("NAME")
                    .help("The name of the protocol")
                    .possible_value("echo")
                    .possible_value("memcache")
                    .possible_value("ping")
                    .possible_value("redis")
                    .possible_value("redis-inline")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("request-ratelimit")
                    .long("request-ratelimit")
                    .value_name("Per-second")
                    .help("Ratelimit for requests per-second")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("request-timeout")
                    .long("request-timeout")
                    .value_name("Microseconds")
                    .help("Base timeout for requests")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("connect-ratelimit")
                    .long("connect-ratelimit")
                    .value_name("Per-second")
                    .help("Ratelimit for connects per-second")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("connect-timeout")
                    .long("connect-timeout")
                    .value_name("Microseconds")
                    .help("Base timeout for connects")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("close-rate")
                    .long("close-rate")
                    .value_name("Per-second")
                    .help("Rate of connections/s that should be client closed")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("tcp-nodelay")
                    .long("tcp-nodelay")
                    .help("Sets the TCP NODELAY socket option")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("warmup-hitrate")
                    .long("warmup-hitrate")
                    .value_name("[0.0-1.0]")
                    .help("Run warmup until hitrate reaches target")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("waterfall")
                    .long("waterfall")
                    .value_name("FILE")
                    .help("Render request latency PNG to file")
                    .takes_value(true),
            );

        #[cfg(feature = "tls")]
        let app = {
            app.arg(
                Arg::with_name("tls-key")
                    .long("tls-key")
                    .value_name("File")
                    .help("Client key for TLS authentication")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("tls-cert")
                    .long("tls-cert")
                    .value_name("File")
                    .help("Client certificate for TLS authentication")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("tls-ca")
                    .long("tls-ca")
                    .value_name("File")
                    .help("Certificate Authority for TLS authentication")
                    .takes_value(true),
            )
        };

        let matches = app.get_matches();

        let mut config = if let Some(file) = matches.value_of("config") {
            Config::load_from_file(file)
        } else {
            println!("NOTE: using builtin base configuration");
            Default::default()
        };

        if let Some(listen) = matches.value_of("listen") {
            let _ = listen.parse::<SocketAddr>().unwrap_or_else(|_| {
                println!("ERROR: listen address is malformed");
                std::process::exit(1);
            });
            config.general.set_listen(Some(listen.to_string()));
        }

        if let Some(clients) = parse_numeric_arg(&matches, "clients") {
            config.general.set_clients(clients);
        }

        if let Some(interval) = parse_numeric_arg(&matches, "interval") {
            config.general.set_interval(interval);
        }

        if let Some(windows) = parse_numeric_arg(&matches, "windows") {
            config.general.set_windows(Some(windows));
        }

        if let Some(poolsize) = parse_numeric_arg(&matches, "poolsize") {
            config.general.set_poolsize(poolsize);
        }

        if let Some(request_ratelimit) = parse_numeric_arg(&matches, "request-ratelimit") {
            config
                .general
                .set_request_ratelimit(Some(request_ratelimit));
        }

        if let Some(request_timeout) = parse_numeric_arg(&matches, "request-timeout") {
            config.general.set_request_timeout(request_timeout);
        }

        if let Some(connect_ratelimit) = parse_numeric_arg(&matches, "connect-ratelimit") {
            config
                .general
                .set_connect_ratelimit(Some(connect_ratelimit));
        }

        if let Some(connect_timeout) = parse_numeric_arg(&matches, "connect-timeout") {
            config.general.set_connect_timeout(connect_timeout);
        }

        if let Some(close_rate) = parse_numeric_arg(&matches, "close-rate") {
            config.general.set_close_rate(Some(close_rate));
        }

        if let Some(warmup_hitrate) = parse_float_arg(&matches, "warmup-hitrate") {
            if warmup_hitrate > 1.0 {
                println!("ERROR: warmup-hitrate is greater than 1.0");
                std::process::exit(1);
            }
            if warmup_hitrate < 0.0 {
                println!("ERROR: warmup-hitrate is less than 0.0");
                std::process::exit(1);
            }
            config.general.set_warmup_hitrate(Some(warmup_hitrate));
        }

        if matches.is_present("endpoint") {
            let mut endpoints = Vec::new();

            for endpoint in matches.values_of("endpoint").unwrap() {
                let mut addrs = endpoint.to_socket_addrs().unwrap_or_else(|_| {
                    println!("ERROR: endpoint address is malformed: {}", endpoint);
                    std::process::exit(1);
                });
                addrs.next().unwrap_or_else(|| {
                    println!("ERROR: failed to resolve address: {}", endpoint);
                    std::process::exit(1);
                });
                endpoints.push(endpoint.to_string());
            }

            config.general.set_endpoints(Some(endpoints));
        }

        config
            .general
            .set_logging(match matches.occurrences_of("verbose") {
                0 => Level::Info,
                1 => Level::Debug,
                _ => Level::Trace,
            });

        if matches.is_present("service") {
            config.general.set_windows(None);
        }

        if matches.is_present("tcp-nodelay") {
            config.general.set_tcp_nodelay(true);
        }

        if let Some(protocol) = matches.value_of("protocol") {
            config.general.set_protocol(match protocol {
                "memcache" => Protocol::Memcache,
                "redis" => Protocol::RedisResp,
                "redis-inline" => Protocol::RedisInline,
                "echo" => Protocol::Echo,
                "ping" => Protocol::Ping,
                _ => {
                    fatal!("unknown protocol: {}", protocol);
                }
            });
        }

        #[cfg(feature = "tls")]
        {
            if let Some(tls_key) = matches.value_of("tls-key") {
                config.general.set_tls_key(Some(tls_key.to_string()));
            }
            if let Some(tls_ca) = matches.value_of("tls-ca") {
                config.general.set_tls_ca(Some(tls_ca.to_string()));
            }
            if let Some(tls_cert) = matches.value_of("tls-cert") {
                config.general.set_tls_cert(Some(tls_cert.to_string()));
            }
        }

        if let Some(waterfall) = matches.value_of("waterfall") {
            config.general.set_waterfall(Some(waterfall.to_string()));
        }

        config
    }

    /// the duration of each integration window in seconds
    pub fn interval(&self) -> usize {
        self.general.interval()
    }

    /// the number of integration periods to run for
    pub fn windows(&self) -> Option<usize> {
        self.general.windows()
    }

    /// the number of client threads to launch
    pub fn clients(&self) -> usize {
        self.general.clients()
    }

    /// the number of connections per-endpoint for each client
    pub fn poolsize(&self) -> usize {
        self.general.poolsize()
    }

    /// get listen address
    pub fn listen(&self) -> Option<SocketAddr> {
        self.general
            .listen()
            .map(|v| v.to_socket_addrs().unwrap().next().unwrap())
    }

    /// get logging level
    pub fn logging(&self) -> Level {
        self.general.logging()
    }

    pub fn endpoints(&self) -> Vec<SocketAddr> {
        let mut endpoints = Vec::new();
        let list = self.general.endpoints().unwrap();
        for endpoint in list {
            endpoints.push(endpoint.to_socket_addrs().unwrap().next().unwrap());
        }
        endpoints
    }

    pub fn protocol(&self) -> Protocol {
        self.general.protocol()
    }

    pub fn request_ratelimit(&self) -> Option<usize> {
        self.general.request_ratelimit()
    }

    pub fn request_timeout(&self) -> usize {
        self.general.request_timeout()
    }

    pub fn connect_timeout(&self) -> usize {
        self.general.connect_timeout()
    }

    pub fn connect_ratelimit(&self) -> Option<usize> {
        self.general.connect_ratelimit()
    }

    pub fn close_rate(&self) -> Option<usize> {
        self.general.close_rate()
    }

    pub fn tcp_nodelay(&self) -> bool {
        self.general.tcp_nodelay()
    }

    #[cfg(feature = "tls")]
    pub fn tls_key(&self) -> Option<String> {
        self.general.tls_key()
    }

    #[cfg(feature = "tls")]
    pub fn tls_cert(&self) -> Option<String> {
        self.general.tls_cert()
    }

    #[cfg(feature = "tls")]
    pub fn tls_ca(&self) -> Option<String> {
        self.general.tls_ca()
    }

    pub fn warmup_hitrate(&self) -> Option<f64> {
        self.general.warmup_hitrate()
    }

    pub fn waterfall(&self) -> Option<String> {
        self.general.waterfall()
    }

    fn load_from_file(file: &str) -> Config {
        let mut file = std::fs::File::open(file).expect("failed to open workload file");
        let mut content = String::new();
        file.read_to_string(&mut content).expect("failed to read");
        let toml: Config = toml::from_str(&content).expect("failed to parse toml");
        toml
    }

    pub fn generator(&self) -> Generator {
        let mut keyspaces = Vec::new();
        for keyspace in &self.keyspace {
            keyspaces.push(keyspace.generator());
        }
        Generator { keyspaces }
    }

    pub fn print(&self) {
        info!("-----");
        info!("Protocol: {:?}", self.protocol());
        let endpoints = self.endpoints();
        for endpoint in &endpoints {
            info!("Config: Endpoint: {}", endpoint,);
        }
        info!(
            "Config: Clients: {} Poolsize: {} Endpoints: {}",
            self.clients(),
            self.poolsize(),
            endpoints.len(),
        );
        info!(
            "Config: Connections: Per-Endpoint: {} Per-Client: {} Total: {}",
            self.clients() * self.poolsize(),
            self.poolsize() * endpoints.len(),
            self.clients() * self.poolsize() * endpoints.len(),
        );
        info!(
            "Config: Ratelimit (/s): Connect: {} Request: {}",
            self.connect_ratelimit()
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "Unlimited".to_string()),
            self.request_ratelimit()
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "Unlimited".to_string()),
        );
        info!(
            "Config: Timeout (us): Connect: {} Request: {}",
            self.connect_timeout(),
            self.request_timeout(),
        );
        let windows = self
            .windows()
            .map(|v| format!("{}", v))
            .unwrap_or_else(|| "Unlimited".to_string());
        let runtime = if let Some(w) = self.windows() {
            format!("{} seconds", self.interval() * w)
        } else {
            "Unlimited".to_string()
        };
        info!(
            "Config: Interval: {} seconds Windows: {} Runtime: {}",
            self.interval(),
            windows,
            runtime
        );
        for keyspace in &self.keyspace {
            info!(
                "Config: Keyspace: Length: {} Commands: {} Value Sizes: {}",
                keyspace.length,
                keyspace.commands.len(),
                keyspace.values.len()
            );
        }
    }
}

/// a helper function to parse a numeric argument by name from `ArgMatches`
fn parse_numeric_arg(matches: &ArgMatches, key: &str) -> Option<usize> {
    matches.value_of(key).map(|f| {
        f.parse().unwrap_or_else(|_| {
            println!("ERROR: could not parse {}", key);
            process::exit(1);
        })
    })
}

/// a helper function to parse a floating point argument by name from `ArgMatches`
fn parse_float_arg(matches: &ArgMatches, key: &str) -> Option<f64> {
    matches.value_of(key).map(|f| {
        f.parse().unwrap_or_else(|_| {
            println!("ERROR: could not parse {}", key);
            process::exit(1);
        })
    })
}
