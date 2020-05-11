# rpc-perf

rpc-perf was created to help measure the performance of caching systems. We've found this tool to be
useful for validating performance of cache backends, effects of kernel version and system tuning, as
well as testing new hardware platforms and network changes.

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

rpc-perf is built through the `cargo` command which ships with rust. If you don't have Rust
installed, you can use [rustup][rustup] to manage your Rust installation. Otherwise, follow the
instructions on [rust-lang.org](https://rust-lang.org) to get Rust and Cargo installed.
rpc-perf targets stable Rust.

### Build from source

With rust installed, clone this repo, and cd into this folder:

```shell
git clone https://github.com/twitter/rpc-perf.git
cd rpc-perf/rpc-perf
cargo build --release
```

If you need TLS support, you'll need to use nightly Rust:

```shell
git clone https://github.com/twitter/rpc-perf.git
cd rpc-perf/rpc-perf
rustup override set nightly
cargo build --release --features tls
```

This will produce a binary at `../target/release/rpc-perf` which can be run in-place or copied to a
more convenient location on your system.

## Configuration

rpc-perf is configured through a combination of a TOML config file and command line parameters. If 
an option is specified in both the config file and on the command line, the command line wins. See
the `--help` and the example configurations in `rpc-perf/configs` to learn more about configuration.

## Sample Usage

**BEWARE** use caution when running rpc-perf

```shell
# display help
rpc-perf --help

# use a config file and specify an endpoint
rpc-perf --config some_config.toml --endpoint 127.0.0.1:11211

# use a config file and override the request rate
rpc-perf --config some_config.toml --endpoint 127.0.0.1:11211 --request-rate 200000

# use a config file and override the protocol
rpc-perf --config some_config.toml --endpoint 127.0.0.1:6379 --protocol redis

# generate a waterfall plot of request latency
rpc-perf --config some_config.toml --endpoint 127.0.0.1:11211 --interval 60 --windows 5 --waterfall waterfall.png
```

## Stats Port

Use the `--listen` or `listen` option in the `general` section of your TOML
config to enable HTTP based stats exposition. This will allow for scraping the
metrics provided by rpc-perf into Prometheus or other compatible observability
stack. A typical use case would be for long-running tests where you wish to
correlate client metrics with system or service metrics.

## Admin Port

Use the `--admin` or `admin` option in the `general` section of your TOML config
to enable a HTTP admin endpoint. You can use this endpoint to change the request
ratelimit using `PUT` requests. For example, if configured with port `40404` as
the admin port: ```curl -X PUT -d 100 127.0.0.1:40404/ratelimit/request``` would
update the current rate to 100 requests per second. To use this, you must set a
request ratelimit when launching rpc-perf.

## Practices

* Start with a short test before moving on to tests spanning larger periods of time
* If comparing latency between two setups, be sure to set a ratelimit that's achievable on both
* Keep `--clients` below the number of cores on the machine generating workload
* Increase `--poolsize` as necessary to simulate production-like connection numbers
* You may need to use multiple machines to generate enough workload and/or connections to the target
* Log your configuration and results to make repeating and sharing experiments easy
* Use waterfalls to help visualize latency distribution over time and see anomalies

## Features

* high-resolution latency metrics
* supports memcache and redis protocols
* [mio][mio] for async networking
* optional waterfall visualization of latencies
* powerful workload configuration

## Support

Create a [new issue](https://github.com/twitter/rpc-perf/issues/new) on GitHub.

## Contributing

We feel that a welcoming community is important and we ask that you follow
Twitter's [Open Source Code of Conduct] in all interactions with the community.

## Authors

* Brian Martin <bmartin@twitter.com>

A full list of [contributors] can be found on GitHub.

Follow [@TwitterOSS](https://twitter.com/twitteross) on Twitter for updates.

## License

Copyright 2015-2019 Twitter, Inc.

Licensed under the Apache License, Version 2.0:
https://www.apache.org/licenses/LICENSE-2.0

## Security Issues?

Please report sensitive security issues via Twitter's bug-bounty program
(https://hackerone.com/twitter) rather than GitHub.

[contributors]: https://github.com/twitter/rpc-perf/graphs/contributors?type=a
[mio]: https://github.com/carllerche/mio
[Open Source Code of Conduct]: https://github.com/twitter/code-of-conduct/blob/master/code-of-conduct.md
[rustlang]: https://rust-lang.org/
[rustup]: https://rustup.rs

