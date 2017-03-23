#!/bin/bash
set -exo pipefail

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

run_tests() {
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
  rm -rf ARTIFACT
}

check_misc_targets() {
  # Build version without autoupgrade
  docker_run cargo build --no-default-features --verbose
}

build_lal_tarball() {
  docker_run cargo build --release --verbose
  mkdir -p musl/bin
  mkdir -p musl/share/lal/configs
  cp target/x86_64-unknown-linux-musl/release/lal musl/bin
  cp lal.complete* musl/share/lal/
  cp configs/* musl/share/lal/configs/
  tar czf lal.tar -C musl .
  rm -rf musl/
  echo "Created lal tarball with contents:"
  tar tvf lal.tar
}

create_lal_upload() {
  # Upload to a folder on artifactory equal to the Cargo.toml version
  lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}')
  # But only if that folder doesn't already exist
  buildurl="http://engci-maven.cisco.com/artifactory/api/storage/CME-release/lal"
  if curl -s "${buildurl}" | grep -q "$lalversion"; then
      echo "lal version already uploaded - stopping" # don't want to overwrite
  else
    echo "Packaging new lal version"
    mkdir "ARTIFACT/${lalversion}" -p
    cp lal.tar "ARTIFACT/${lalversion}/"
    # Update the latest package
    cp "ARTIFACT/${lalversion}" "ARTIFACT/latest" -R
  fi
}

main() {
  # build in the currently available muslrust container
  local -r container="$(docker images -q edonusdevelopers/muslrust | head -n 1)"
  run_tests
  check_misc_targets
  build_lal_tarball
  create_lal_upload
  rm lal.tar
}

main
