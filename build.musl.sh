#!/bin/bash


docker pull clux/muslrust:1.9.0-nightly-2016-03-24
docker tag clux/muslrust clux/muslrust:latest
docker run -v "$PWD:/volume" -w /volume \
  -t clux/muslrust cargo build --release --verbose

mkdir -p musl/bin
cp target/x86_64-unknown-linux-musl/release/lal musl/bin
tar czf lal.musl.tar -C musl .
rm -rf musl/

rm -rf ARTIFACT
mkdir ARTIFACT -p
lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}')
cp target/x86_64-unknown-linux-musl/release/lal  "ARTIFACT/lal-${lalversion}"
