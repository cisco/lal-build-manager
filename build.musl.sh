#!/bin/bash
set -ex
container="edonusdevelopers/muslrust:1.10.0-2016-07-25"

docker_run() {
  # shellcheck disable=SC2068
  docker run -u lal -v "$PWD:/volume" -w /volume -t ${container} $@
}

# compile test executable
docker_run cargo build --test testmain

# ensure we don't overwrite a buildmachines config
# back it up, then restore on EXIT
[ -f ~/.lal/config ] && cp ~/.lal/config ./bupconfig
restore_config() {
  [ -f bupconfig ] && mv bupconfig ~/.lal/config
}
trap restore_config EXIT

# run tests
./target/x86_64-unknown-linux-musl/debug/testmain-*

# compile lal
docker_run cargo build --release --verbose

# create release tarball in right dir structure for artifactory
lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}')

buildurl="http://engci-maven.cisco.com/artifactory/api/storage/CME-release/lal"
if curl -s "${buildurl}" | grep -q "$lalversion"; then
    echo "lal version already uploaded - stopping" # don't want to overwrite
else
  echo "Packaging new lal version"
  mkdir -p musl/bin
  mkdir -p musl/share/lal/
  cp target/x86_64-unknown-linux-musl/release/lal musl/bin
  cp lal.complete* musl/share/lal/
  tar czf lal.tar -C musl .
  rm -rf musl/
  rm -rf ARTIFACT
  mkdir "ARTIFACT/${lalversion}" -p
  cp lal.tar "ARTIFACT/${lalversion}/"
  # Update the latest package
  cp "ARTIFACT/${lalversion}" "ARTIFACT/latest" -R
fi
