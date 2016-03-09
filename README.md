# rpc-perf - RPC Performance Testing

rpc-perf was created to help measure the performance of caching systems. We've found this tool to be useful for validating performance of cache backends, effects of kernel version and system tuning, as well as testing new hardware platforms and network changes.

**BEWARE** rpc-perf can write to its target and can generate many requests
* *run only* if data in the server can be lost/destroyed/corrupted/etc
* *run only* if you understand the impact of sending high-levels of traffic across your network

**Contents**
* [Build](#build)
* [Configuration](#configuration)
* [Sample Usage](#sample-usage)
* [Sample Output](#sample-output)
* [Practices](#practices)
* [Features](#features)
* [Future Work](#future-work)
* [Contributing](#contributing)

## Build

rpc-perf is built through the `cargo` command which ships with rust. If you don't have rust installed, I recommend using [multirust][1] to manage your rust installation. Otherwise, follow the instructions on [rust-lang.org][2] to get rust and cargo installed. rpc-perf targets stable rust

With rust installed, clone this repo, and cd into this folder:

```shell
git clone https://github.com/twitter/rpc-perf.git
cd rpc-perf
cargo build --release
```

This will produce a binary at `./target/release/rpc-perf` which can be run in-place or copied to a more convenient location on your system.

## Configuration

rpc-perf is configured through a combination of a TOML config file and command line parameters. This provides quick iteration through the command line while allowing more complex configurations to be encoded in a file. Where possible, the command line will override the config file. Be aware of the following rules:

Some configuration is **only** through command line parameters:
* `--server [HOST:PORT]` the target server *is always* required
* `--trace [FILE]` an optional trace file

Some features are **only** accessible through the config file, as they would be difficult to express through the command line:
* multiple workloads

**DO NOT** specify a workload in the config in-combination with the following on the command line:
* `method`
* `rate`
* `bytes`

All other test configuration parameters are available through the TOML config file and/or on the command line. The command line parameter will take precedence when both are specified.

Sample configurations can be found in the `configs` directory of this project. The command line arguments are documented through the `--help` option. Configuration parameters are named the same as the options: eg `--protocol` on the command line and `protocol` in the file are the same

## Sample Usage

**BEWARE** use caution when running rpc-perf

```shell
# display help
./target/release/rpc-perf --help

# memcache get hit
./target/release/rpc-perf --server 127.0.0.1:11211 --protocol memcache --method get --hit

# memcache get miss
./target/release/rpc-perf --server 127.0.0.1:11211 --protocol memcache --method get --flush

# redis get with ratelimit of 50kqps
./target/release/rpc-perf --server 127.0.0.1:6379 --protocol redis --method get --hit --rate 50000

# run the same test against memcache and redis
./target/release/rpc-perf --config configs/default.toml --server 127.0.0.1:11211 --protocol memcache
./target/release/rpc-perf --config configs/default.toml --server 127.0.0.1:6379 --protocol redis
```

## Sample Output

```
$ ./target/release/rpc-perf --config configs/default.toml -s 10.0.0.11:11211 -d 60 -w 1
2016-01-17 20:32:21 INFO  [rpc-perf] rpc-perf 0.2.1 initializing...
2016-01-17 20:32:21 INFO  [rpc-perf] -----
2016-01-17 20:32:21 INFO  [rpc-perf] Config:
2016-01-17 20:32:21 INFO  [rpc-perf] Config: Server: 10.0.0.11:11211 Protocol: memcache
2016-01-17 20:32:21 INFO  [rpc-perf] Config: IP: IP::Any TCP_NODELAY: false
2016-01-17 20:32:21 INFO  [rpc-perf] Config: Threads: 4 Connections: 25
2016-01-17 20:32:21 INFO  [rpc-perf] Config: Windows: 1 Duration: 60
2016-01-17 20:32:21 INFO  [rpc-perf] -----
2016-01-17 20:32:21 INFO  [rpc-perf] Workload:
2016-01-17 20:32:21 INFO  [rpc-perf] Workload 0: Method: set Bytes: 1 Rate: 10000 Hit: false Flush: false
2016-01-17 20:32:21 INFO  [rpc-perf] Workload 1: Method: get Bytes: 1 Rate: 50000 Hit: true Flush: false
2016-01-17 20:32:21 INFO  [rpc-perf] -----
2016-01-17 20:32:21 INFO  [rpc-perf] Connecting...
2016-01-17 20:32:21 INFO  [rpc-perf] Connections: 25 Failures: 0
2016-01-17 20:32:21 INFO  [rpc-perf] Connections: 25 Failures: 0
2016-01-17 20:32:21 INFO  [rpc-perf] Connections: 25 Failures: 0
2016-01-17 20:32:21 INFO  [rpc-perf] Connections: 25 Failures: 0
2016-01-17 20:33:21 INFO  [rpc-perf] -----
2016-01-17 20:33:21 INFO  [rpc-perf] Warmup complete
2016-01-17 20:34:21 INFO  [rpc-perf] -----
2016-01-17 20:34:21 INFO  [rpc-perf] Window: 1
2016-01-17 20:34:21 INFO  [rpc-perf] Requests: 3600001 Ok: 3600001 Miss: 0 Error: 0 Closed: 0
2016-01-17 20:34:21 INFO  [rpc-perf] Rate: 60000 rps Success: 100 % Hitrate: 100 %
2016-01-17 20:34:21 INFO  [rpc-perf] Latency: min: 22245 ns max: 8053903 ns avg: 55236 ns stddev: 69891 ns
2016-01-17 20:34:21 INFO  [rpc-perf] Percentiles: p50: 47485 ns p90: 65792 ns p99: 140130 ns p999: 1094714 ns p9999: 2219836 ns
```

## Practices

* Start with a short test before moving on to tests spanning larger periods of time `--duration 1 --windows 1` makes for a quick smoke test
* When benchmarking for peak throughput, be sure to run enough workers with enough connections to keep them busy sending requests and reading responses. With too few threads, latency will impact throughput. With too many threads, the clients might starve for CPU
* When benchmarking for latency, be sure to ratelimit and compare across a variety of rates. Use `--duration 60` (the default) to latch the histogram at one minute intervals to match up with clients which report percentiles
* Log your configuration and results, this will help you repeat the experiment and compare results reliably

## Features

* high-resolution latency metrics
* supports memcache and redis protocols
* [mio][3] for async networking
* optional trace file for generating heatmaps
* ratelimited workload
* mixed-workload (get/set)

## Future work

* extend command sets for both memcache and redis protocols
* UDP support
* multi-key workload generators
* command log playback

## Contributing

* fork on github
* clone your fork
* create a feature branch
* don't forget to run rustfmt
* push your feature branch
* create a pull request

[1]: https://github.com/brson/multirust
[2]: https://rust-lang.org/
[3]: https://github.com/carllerche/mio