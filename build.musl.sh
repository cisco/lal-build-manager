#!/bin/bash
set -ex

#docker pull edonusdevelopers/muslrust:1.9.0-nightly-2016-04-08
docker run -u lal -v "$PWD:/volume" -w /volume \
  -t edonusdevelopers/muslrust:1.8.0-2016-04-15 cargo build --release --verbose

mkdir -p musl/bin
cp target/x86_64-unknown-linux-musl/release/lal musl/bin
tar czf lal.tar -C musl .
rm -rf musl/

lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}')

buildurl="http://engci-maven.cisco.com/artifactory/api/storage/CME-group/lal "
if curl -s "${buildurl}" | grep -q "$lalversion"; then
    echo "lal version already uploaded - stopping" # don't want to overwrite
else
  echo "Packaging new lal version"
  rm -rf ARTIFACT
  mkdir "ARTIFACT/${lalversion}" -p
  cp lal.tar "ARTIFACT/${lalversion}/"
fi
