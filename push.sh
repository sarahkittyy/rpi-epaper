#!/usr/bin/bash

set -e

if [[ $1 = "debug" ]]; then
	cargo build
	sftp 192.168.0.106 <<<$'lcd target/aarch64-unknown-linux-gnu/debug\n put rpi-epaper'
else
	cargo build --release
	sftp 192.168.0.106 <<<$'lcd target/aarch64-unknown-linux-gnu/release\n put rpi-epaper'
fi
