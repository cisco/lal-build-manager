#!/bin/bash

docker run -v $PWD:/volume -w /volume -t clux/muslrust cargo build --verbose

mkdir -p musl/bin
cp target/x86_64-unknown-linux-musl/debug/lal musl/bin
tar czf lal.musl.tar -C musl .
rm -rf musl/
