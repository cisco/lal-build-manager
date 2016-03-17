#!/bin/bash

docker run -v $PWD:/volume -w /volume -t clux/muslrust cargo build --release

mkdir -p musl/bin
cp target/x86_64-unknown-linux-musl/release/lal musl/bin
tar czf lal.musl.tar -C musl .
rm -rf musl/

# Almost there. Just need to fix a curl thing.
