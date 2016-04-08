#!/bin/bash
set -ex

docker pull edonusdevelopers/muslrust:1.9.0-nightly-2016-04-08
docker tag edonusdevelopers/muslrust:1.9.0-nightly-2016-04-08 edonusdevelopers/muslrust:latest
docker run -u lal -v "$PWD:/volume" -w /volume \
  -t edonusdevelopers/muslrust cargo build --release --verbose

mkdir -p musl/bin
cp target/x86_64-unknown-linux-musl/release/lal musl/bin
tar czf lal.musl.tar -C musl .
rm -rf musl/

rm -rf ARTIFACT
lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}')
mkdir "ARTIFACT/${lalversion}" -p
cp target/x86_64-unknown-linux-musl/release/lal  "ARTIFACT/${lalversion}/"
