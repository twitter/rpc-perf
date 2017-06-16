//  rpc-perf - RPC Performance Testing
//  Copyright 2017 Twitter, Inc
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

use getopts::Options;

pub fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn opts() -> Options {
    let mut opts = Options::new();

    opts.optmulti("s", "server", "server address", "HOST:PORT");
    opts.optopt("t", "threads", "number of threads", "INTEGER");
    opts.optopt("c", "connections", "connections per thread", "INTEGER");
    opts.optopt("d", "duration", "number of seconds per window", "INTEGER");
    opts.optopt("w", "windows", "number of windows in test", "INTEGER");
    opts.optopt(
        "",
        "request-timeout",
        "request timeout in milliseconds",
        "INTEGER",
    );
    opts.optopt(
        "",
        "connect-timeout",
        "connect timeout in milliseconds",
        "INTEGER",
    );
    opts.optopt("p", "protocol", "client protocol", "STRING");
    opts.optopt("", "config", "TOML config file", "FILE");
    opts.optopt("", "listen", "listen address for stats", "HOST:PORT");
    opts.optopt("", "trace", "write histogram data to file", "FILE");
    opts.optopt("", "waterfall", "output waterfall PNG", "FILE");
    opts.optflag("", "tcp-nodelay", "enable tcp nodelay");
    opts.optflag("", "flush", "flush cache prior to test");
    opts.optflag("", "ipv4", "force IPv4 only");
    opts.optflag("", "ipv6", "force IPv6 only");
    opts.optflag("", "service", "run continuously");
    opts.optflag("", "version", "show version and exit");
    opts.optflagmulti("v", "verbose", "verbosity (stacking)");
    opts.optflag("h", "help", "print this help menu");

    opts
}
