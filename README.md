# rpc-perf - RPC Performance Testing

rpc-perf was created to help measure the performance of caching systems. We've found this tool to be useful for validating performance of cache backends, effects of kernel version and system tuning, as well as testing new hardware platforms and network changes.

**BEWARE** rpc-perf can write to its target and can generate many requests
* *run only* if data in the server can be lost/destroyed/corrupted/etc
* *run only* if you understand the impact of sending high-levels of traffic across your network

**Contents**
* [Getting rpc-perf](#getting-rpc-perf)
* [Configuration](#configuration)
* [Sample Usage](#sample-usage)
* [Sample Output](#sample-output)
* [Practices](#practices)
* [Features](#features)
* [Future Work](#future-work)
* [Contributing](#contributing)

## Getting rpc-perf

rpc-perf is built through the `cargo` command which ships with rust. If you don't have rust installed, I recommend using [multirust][1] to manage your rust installation. Otherwise, follow the instructions on [rust-lang.org][2] to get rust and cargo installed. rpc-perf targets stable rust.

### Build from source

With rust installed, clone this repo, and cd into this folder:

```shell
git clone https://github.com/twitter/rpc-perf.git
cd rpc-perf
cargo build --release
```

If you want to use the x86_64 TSC for timing, using nightly rust:

```shell
git clone https://github.com/twitter/rpc-perf.git
cd rpc-perf
cargo build --release --features asm
```

This will produce a binary at `./target/release/rpc-perf` which can be run in-place or copied to a more convenient location on your system.

## Configuration

rpc-perf is configured through a combination of a TOML config file and command line parameters. The workload itself is always specified in the config file. Some runtime parameters are passed on the command line. Where possible, the command line can override the configuration file. For example, the protocol can be overriden to test memcache or redis with the same workload.

Some configuration is **only** through command line parameters:
* `--server [HOST:PORT]` the target server *is always* required. You may specify more than one.
* `--trace [FILE]` an optional latency trace file
* `--waterfall [FILE]` an optional PNG waterfall plot

All other test configuration parameters are available through the TOML config file and/or on the command line. The command line parameter will take precedence when both are specified.

Sample configurations can be found in the `configs` directory of this project. The command line arguments are documented through the `--help` option. Configuration parameters are named the same as the options: eg `--protocol` on the command line and `protocol` in the file are the same

## Sample Usage

**BEWARE** use caution when running rpc-perf

```shell
# display help
./target/release/rpc-perf --help

# memcache get hit
./target/release/rpc-perf --config configs/hotkey_hit.toml --server 127.0.0.1:11211

# memcache get miss
./target/release/rpc-perf --config configs/hotkey_hit.toml --server 127.0.0.1:11211 --flush

# redis mixed workload
./target/release/rpc-perf ---config configs/mixed_workload.toml -server 127.0.0.1:6379 --protocol redis

# run the same test against memcache and redis
./target/release/rpc-perf --config configs/default.toml --server 127.0.0.1:11211 --protocol memcache
./target/release/rpc-perf --config configs/default.toml --server 127.0.0.1:6379 --protocol redis
```

## Sample Output

```
$ rpc-perf --server 127.0.0.1:11211 --config hotkey_hit.toml --windows 1
2016-03-25 15:00:37 INFO  [rpc-perf] rpc-perf 1.0.0-nightly.20160324 initializing...
2016-03-25 15:00:37 INFO  [rpc-perf] -----
2016-03-25 15:00:37 INFO  [rpc-perf] Config:
2016-03-25 15:00:37 INFO  [rpc-perf] Config: Server: 127.0.0.1:11211 Protocol: memcache
2016-03-25 15:00:37 INFO  [rpc-perf] Config: IP: IP::Any TCP_NODELAY: false
2016-03-25 15:00:37 INFO  [rpc-perf] Config: Threads: 1 Connections: 1
2016-03-25 15:00:37 INFO  [rpc-perf] Config: Windows: 1 Duration: 60
2016-03-25 15:00:37 INFO  [rpc-perf] -----
2016-03-25 15:00:37 INFO  [rpc-perf] Workload:
2016-03-25 15:00:37 INFO  [rpc-perf] Workload 0: Method: get Rate: 0
2016-03-25 15:00:37 INFO  [rpc-perf] Parameter: Static { size: 1, seed: 0 }
2016-03-25 15:00:37 INFO  [rpc-perf] Workload 1: Method: set Rate: 1
2016-03-25 15:00:37 INFO  [rpc-perf] Parameter: Static { size: 1, seed: 0 }
2016-03-25 15:00:37 INFO  [rpc-perf] Parameter: Random { size: 128, regenerate: false }
2016-03-25 15:00:37 INFO  [rpc-perf] -----
2016-03-25 15:00:37 INFO  [rpc-perf] Connecting...
2016-03-25 15:00:37 INFO  [rpc-perf] Client: 0
2016-03-25 15:00:37 INFO  [rpc-perf] Connections: 1 Failures: 0
2016-03-25 15:01:37 INFO  [rpc-perf] -----
2016-03-25 15:01:37 INFO  [rpc-perf] Warmup complete
2016-03-25 15:02:37 INFO  [rpc-perf] -----
2016-03-25 15:02:37 INFO  [rpc-perf] Window: 1
2016-03-25 15:02:37 INFO  [rpc-perf] Requests: 986233 Ok: 986233 Miss: 0 Error: 0 Closed: 0
2016-03-25 15:02:37 INFO  [rpc-perf] Rate: 16437.21 rps Success: 100.00 % Hitrate: 100.00 %
2016-03-25 15:02:38 INFO  [rpc-perf] Latency: min: 24103 ns max: 37876243 ns avg: 49437 ns stddev: 83165 ns
2016-03-25 15:02:38 INFO  [rpc-perf] Percentiles: p50: 47393 ns p90: 55116 ns p99: 75688 ns p999: 224553 ns p9999: 3929854 ns
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
* optional trace file for further analysis
* optional waterfall visualization of latencies
* ratelimited workload
* mixed-workload (get/set/...)

## Future work

* extend command sets for both memcache and redis protocols
* UDP support
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
