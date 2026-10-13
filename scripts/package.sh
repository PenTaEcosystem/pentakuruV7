#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
mkdir -p dist
cp target/release/pentakuru dist/pentakuru
platform="$(uname -s)"
arch="$(uname -m)"
tar -czf "dist/PentaKuru-${platform}-${arch}.tar.gz" -C dist pentakuru -C .. README.md LICENSE assets/fonts/OFL.txt
