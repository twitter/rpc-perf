#!/bin/bash

set -e

export RUST_BACKTRACE=1

cargo check
cargo build --release
cargo test --release
