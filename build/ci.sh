#!/bin/bash

set -e

# try to install or use existing sccache
if sccache --version > /dev/null; then
	echo "Using existing sccache"
	export RUSTC_WRAPPER="sccache"
	sccache --version
elif cargo install sccache; then
	echo "Installed sccache"
	export RUSTC_WRAPPER="sccache"
	sccache --version
else
	echo "Building without sccache"
fi

export RUST_BACKTRACE=1

if [ -z ${FEATURES} ]; then
	cargo build
	cargo test
	cargo build --release
	cargo test --release
elif [ -n ${PACKAGE} ]; then
	# features can't be enabled in the virtual manifest, cd into package
	cd ${PACKAGE}
	cargo build --features ${FEATURES}
	cargo test --features ${FEATURES}
	cargo build --release --features ${FEATURES}
	cargo test --release --features ${FEATURES}
else
	echo "Specified FEATURES for build, but no PACKAGE"
	exit 1
fi
