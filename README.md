# rpc-perf - RPC Performance Testing

rpc-perf was created to help measure the performance of caching systems. We've found this tool to be useful for validating performance of cache backends, effects of kernel version and system tuning, as well as testing new hardware platforms and network changes.

## Build

To use `rpc-perf`, with rust installed, clone and cd into this folder:

```shell
cargo build --release;
```

This will produce a binary at ./target/release/rpc-perf

## Usage

BEWARE!: rpc-perf can and will write to your server. Only use against a server if the data can be lost/destroyed/corrupted/...

There are three ways to use rpc-perf:
* specify all options on the command line. This provides limited access to the functionality (single workload)
* specify most options via a TOML config file. This provides access to advanced functionality (mixed workloads)
* specify a TOML config and override options on command line. This allows you to change some aspects of the config without editing the config.

NOTE: Some of the options may conflict with usage of a config file. In such cases, rpc-perf will abort.


```shell
# display help
./target/release/rpc-perf --help
# memcache get hit
./target/release/rpc-perf --server 127.0.0.1:11211 --protocol memcache --method get --hit
# memcache get miss
./target/release/rpc-perf --server 127.0.0.1:11211 --protocol memcache --method get --flush
# redis get with ratelimit of 50kqps
./target/release/rpc-perf --server 127.0.0.1:6379 --protocol redis --method get --hit --rate 50000
```

## Features

* high-resolution latency metrics
* supports memcache and redis protocols
* mio for async networking
* optional trace file for generating heatmaps
* ratelimited workload
* mixed-workload (get/set)

## Future work

* extend command sets for both memcache and redis protocols
* UDP support
* multi-key workload generators
* command log playback
