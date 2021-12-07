// Copyright 2021 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use core::time::Duration;
use serde_derive::*;
use serde_json::Value as JsonValue;
use std::io::Read;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use zookeeper::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    general: General,
    target: Target,
    #[serde(default)]
    connection: Connection,
    #[serde(default)]
    request: Request,
    tls: Option<Tls>,
    keyspace: Vec<Keyspace>,
}

impl ConfigFile {
    pub fn general(&self) -> General {
        self.general.clone()
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    pub fn request(&self) -> Request {
        self.request
    }

    pub fn tls(&self) -> Option<Tls> {
        self.tls.clone()
    }

    pub fn keyspaces(&self) -> Vec<Keyspace> {
        self.keyspace.clone()
    }

    pub fn target(&self) -> Target {
        self.target.clone()
    }

    pub fn load_from_file(filename: &str) -> Self {
        let mut file = std::fs::File::open(filename).expect("failed to open workload file");
        let mut content = String::new();
        file.read_to_string(&mut content).expect("failed to read");
        let toml = toml::from_str(&content);
        match toml {
            Ok(toml) => toml,
            Err(e) => {
                println!("Failed to parse TOML config: {}", filename);
                println!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn default_interval() -> usize {
    60
}

fn default_windows() -> usize {
    5
}

fn zero() -> usize {
    0
}

fn one() -> usize {
    1
}

fn default_nodelay() -> bool {
    false
}

fn alphanumeric() -> FieldType {
    FieldType::Alphanumeric
}

#[derive(Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum FieldType {
    Alphanumeric,
    U32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Protocol {
    Ping,
    Echo,
    Memcache,
    Redis,
    RedisInline,
    RedisResp,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct General {
    protocol: Protocol,
    #[serde(default = "default_interval")]
    interval: usize,
    #[serde(default = "default_windows")]
    windows: usize,
    #[serde(default = "one")]
    threads: usize,
    #[serde(default)]
    service: bool,
    admin: Option<String>,
}

impl General {
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.interval as u64)
    }

    pub fn windows(&self) -> Option<usize> {
        if self.service {
            None
        } else {
            Some(self.windows)
        }
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn admin(&self) -> Option<String> {
        self.admin.clone()
    }
}

#[derive(Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum RatelimitModel {
    Smooth,
    Uniform,
    Normal,
}

#[derive(Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    #[serde(default = "one")]
    poolsize: usize,
    ratelimit: Option<usize>,
    ratelimit_model: Option<RatelimitModel>,
    reconnect: Option<usize>,
    #[serde(default = "default_nodelay")]
    tcp_nodelay: bool,
    timeout: Option<usize>,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            poolsize: 1,
            ratelimit: None,
            ratelimit_model: None,
            reconnect: None,
            tcp_nodelay: false,
            timeout: None,
        }
    }
}

impl Connection {
    pub fn ratelimit(&self) -> Option<usize> {
        self.ratelimit
    }

    pub fn ratelimit_model(&self) -> rustcommon_ratelimiter::Refill {
        match self.ratelimit_model {
            None | Some(RatelimitModel::Smooth) => rustcommon_ratelimiter::Refill::Smooth,
            Some(RatelimitModel::Uniform) => rustcommon_ratelimiter::Refill::Uniform,
            Some(RatelimitModel::Normal) => rustcommon_ratelimiter::Refill::Normal,
        }
    }

    pub fn reconnect(&self) -> Option<usize> {
        self.reconnect
    }

    pub fn poolsize(&self) -> usize {
        self.poolsize
    }

    pub fn tcp_nodelay(&self) -> bool {
        self.tcp_nodelay
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Keyspace {
    #[serde(default = "one")]
    length: usize,
    #[serde(default = "one")]
    weight: usize,
    commands: Vec<Command>,
    #[serde(default)]
    inner_keys: Vec<InnerKey>,
    #[serde(default)]
    values: Vec<Value>,
    #[serde(default = "zero")]
    ttl: usize,
    #[serde(default = "alphanumeric")]
    key_type: FieldType,
    #[serde(default = "one")]
    batch_size: usize,
}

impl Keyspace {
    pub fn length(&self) -> usize {
        self.length
    }

    pub fn weight(&self) -> usize {
        self.weight
    }

    pub fn inner_keys(&self) -> Vec<InnerKey> {
        self.inner_keys.clone()
    }

    pub fn commands(&self) -> Vec<Command> {
        self.commands.clone()
    }

    pub fn values(&self) -> Vec<Value> {
        self.values.clone()
    }

    pub fn ttl(&self) -> usize {
        self.ttl
    }

    pub fn key_type(&self) -> FieldType {
        self.key_type
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

#[derive(Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Verb {
    /// Sends a simple 'ping' to a pingserver.
    Ping,
    /// Sends a payload with a CRC to an echo server and checks for corruption.
    Echo,
    /// Simple key-value get which reads the value for one or more keys
    /// depending on the batch size.
    Get,
    /// Simple key-value set which will overwrite the value for a key.
    Set,
    /// Remove a key.
    Delete,
    /// Hash get, reads the value for a field within the hash stored at the key.
    Hget,
    /// Hash set, set the value for a field within the hash stored at the key.
    Hset,
    /// Hash set non-existing, set the value for a field within the hash stored
    /// at the key only if the field does not exist.
    Hsetnx,
}

#[derive(Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub struct Command {
    verb: Verb,
    #[serde(default = "one")]
    weight: usize,
}

impl Command {
    pub fn verb(&self) -> Verb {
        self.verb
    }

    pub fn weight(&self) -> usize {
        self.weight
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct InnerKey {
    length: usize,
    #[serde(default = "one")]
    weight: usize,
    #[serde(default = "alphanumeric")]
    field_type: FieldType,
}

impl InnerKey {
    pub fn weight(&self) -> usize {
        self.weight
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn field_type(&self) -> FieldType {
        self.field_type
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Value {
    length: usize,
    #[serde(default = "one")]
    weight: usize,
    #[serde(default = "alphanumeric")]
    field_type: FieldType,
}

impl Value {
    pub fn weight(&self) -> usize {
        self.weight
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn field_type(&self) -> FieldType {
        self.field_type
    }
}

#[derive(Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub struct Request {
    timeout: Option<usize>,
    ratelimit: Option<usize>,
    ratelimit_model: Option<RatelimitModel>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            ratelimit: None,
            ratelimit_model: None,
            timeout: None,
        }
    }
}

impl Request {
    pub fn ratelimit(&self) -> Option<usize> {
        self.ratelimit
    }

    pub fn ratelimit_model(&self) -> rustcommon_ratelimiter::Refill {
        match self.ratelimit_model {
            None | Some(RatelimitModel::Smooth) => rustcommon_ratelimiter::Refill::Smooth,
            Some(RatelimitModel::Uniform) => rustcommon_ratelimiter::Refill::Uniform,
            Some(RatelimitModel::Normal) => rustcommon_ratelimiter::Refill::Normal,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Tls {
    ca: String,
    cert: String,
    key: String,
    verify: bool,
}

impl Tls {
    pub fn ca(&self) -> String {
        self.ca.clone()
    }

    pub fn cert(&self) -> String {
        self.cert.clone()
    }

    pub fn key(&self) -> String {
        self.key.clone()
    }

    pub fn verify(&self) -> bool {
        self.verify
    }
}

struct ExitWatcher;
impl Watcher for ExitWatcher {
    fn handle(&self, _event: WatchedEvent) {
        std::process::exit(2);
    }
}

#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Target {
    endpoints: Vec<String>,
    zk_path: Option<String>,
    zk_server: Option<String>,
    zk_endpoint_name: Option<String>,
}

impl Target {
    pub fn endpoints(&self) -> Vec<SocketAddr> {
        if self.zk_path.is_some() && self.zk_server.is_some() && self.zk_endpoint_name.is_some() {
            let zk_endpoint_name = self.zk_endpoint_name.as_deref().unwrap();
            let mut ret = Vec::new();
            let zk = ZooKeeper::connect(
                self.zk_server.as_ref().unwrap(),
                Duration::from_secs(15),
                ExitWatcher,
            )
            .unwrap();
            let children = zk
                .get_children(self.zk_path.as_ref().unwrap(), true)
                .unwrap();
            for child in children {
                let child_path = format!("{}/{}", self.zk_path.as_ref().unwrap(), child);
                let data = zk.get_data(&child_path, true).unwrap();
                let data = std::str::from_utf8(&data.0).unwrap();
                let entry: JsonValue = serde_json::from_str(data).unwrap();
                let host = &entry["additionalEndpoints"][zk_endpoint_name]["host"];
                let host = host.to_string();
                let host_parts: Vec<&str> = host.split('"').collect();
                let port = &entry["additionalEndpoints"][zk_endpoint_name]["port"];
                if let Some(host) = host_parts.get(1) {
                    let host = format!("{}:{}", host, port);
                    if let Ok(mut addrs) = host.to_socket_addrs() {
                        if let Some(socket_addr) = addrs.next() {
                            ret.push(socket_addr);
                        }
                    }
                }
            }
            ret
        } else {
            let mut ret = Vec::new();
            for host in &self.endpoints {
                if let Ok(mut addrs) = host.to_socket_addrs() {
                    if let Some(socket_addr) = addrs.next() {
                        ret.push(socket_addr);
                    }
                }
            }
            ret
        }
    }
}
