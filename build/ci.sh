#!/bin/bash

set -e

export RUST_BACKTRACE=1

if [ -z "${FEATURES}" ]; then
	cargo check
	cargo build --release
	cargo test --release
else
	cd rpc-perf
	cargo check
	cargo build --release --features "${FEATURES}"
	cargo test --release --features "${FEATURES}"
fi
