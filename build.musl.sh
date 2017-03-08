#!/bin/bash
set -exo pipefail
# build in the currently available muslrust container
container="$(docker images -q edonusdevelopers/muslrust | head -n 1)"

cargo_cache() {
  # Allow using a build cache when not on the build workers
  if grep -q "CoreOS" /etc/os-release; then
    # normal fresh build with lal user
    echo "-u lal"
  else
    # cached /root/.cargo (permission errors if we use -u lal here)
    # this is still not perfect - it screws with the permissions in $PWD
    # so switching between debug and musl builds forces some rebuildiness
    # but it's better than nothing when repeat-building musl
    echo "-v cargo-cache:/root/.cargo"
  fi
}

docker_run() {
  local -r cache=$(cargo_cache)
  # shellcheck disable=SC2068,SC2086
  docker run --rm $cache -v "$PWD:/volume" -w /volume -t "${container}" $@
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
  mkdir -p musl/share/lal/configs
  cp target/x86_64-unknown-linux-musl/release/lal musl/bin
  cp lal.complete* musl/share/lal/
  cp configs/* musl/share/lal/configs/
  tar czf lal.tar -C musl .
  rm -rf musl/
  rm -rf ARTIFACT
  mkdir "ARTIFACT/${lalversion}" -p
  cp lal.tar "ARTIFACT/${lalversion}/"
  # Update the latest package
  cp "ARTIFACT/${lalversion}" "ARTIFACT/latest" -R
fi
