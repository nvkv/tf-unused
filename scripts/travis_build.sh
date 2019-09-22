#!/usr/bin/env bash

## TravisCI build script

set -eo pipefail

OS="$1"

if [ "${OS}" != "linux" ] && [ "${OS}" != "osx" ]; then
	echo "OS is not specified of unknown ${OS}"
	exit 1
fi

function install_musl_target() {
	if [ "${OS}" != "linux" ]; then
		echo "Skipping musl target install on non-linux os"
		return 0
	fi
	rustup target add x86_64-unknown-linux-musl
}

function run_tests() {
	cargo test --verbose --all
}

function build_osx_release() {
	cargo build --verbose --all --release
	cp ./target/release/tf-unused .
}

function build_linux_release() {
	RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl
	cp ./target/x86_64-unknown-linux-musl/release/tf-unused .
}

function build() {
	if [ "${OS}" == "linux" ]; then
		build_linux_release
	elif [ "${OS}" == "osx" ]; then
		build_osx_release
	else
		echo "Unknown OS"
		exit 1
	fi
}

function pack_the_binary() {
	if [ ! -f ./tf-unused ]; then
		echo "ERROR: Couldn't locate compiled binary"
		exit 1
	fi
	mkdir -p tf-unused-${OS}
	mv ./tf-unused "./tf-unused-${OS}"
	tar czf "tf-unused-${OS}.tar.gz" "./tf-unused-${OS}"
}

install_musl_target
run_tests
build
pack_the_binary