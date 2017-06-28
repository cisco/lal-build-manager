#!/bin/bash


run_tests() {
  # ensure we don't overwrite your config
  # back it up, then restore on EXIT
  [ -f ~/.lal/config ] && cp ~/.lal/config ./bupconfig
  restore_config() {
    [ -f bupconfig ] && mv bupconfig ~/.lal/config
  }
  trap restore_config EXIT
  export RUST_BACKTRACE=full
  ./OUTPUT/testmain-*
  restore_config
}

main() {
  # build in the currently available muslrust container
  set -e
  if [ ! -d ~/.cargo/registry ]; then
    echo "Ensure you have created a cargo-cache docker volume to speed up subsequent builds"
    echo "If this is your first build, this is normal"
    echo "Otherwise, please 'docker volume create cargo-cache' and ensure it is specified in your lal config"
    echo "Continuing from blank cache..."
  fi
  if [[ $1 == "lal" ]]; then
    mkdir -p OUTPUT/{bin,share/lal/configs}
    cp configs/*.json OUTPUT/share/lal/configs/
    cp lal.complete* OUTPUT/share/lal/
    if [[ $2 == "slim" ]]; then
      (set -x; cargo build --no-default-features --release)
      cp ./target/x86_64-unknown-linux-musl/release/lal OUTPUT/bin/
    elif [[ $2 == "release" ]]; then
      (set -x; cargo build --release)
      cp ./target/x86_64-unknown-linux-musl/release/lal OUTPUT/bin/
    elif [[ $2 == "debug" ]]; then
      (set -x; cargo build)
      cp ./target/x86_64-unknown-linux-musl/debug/lal OUTPUT/bin/
    else
      echo "No such configuration $2 found"
      exit 2
    fi
  elif [[ $1 == "lal-unit-tests" ]]; then
    cargo build --test testmain
    cp ./target/x86_64-unknown-linux-musl/debug/testmain-* OUTPUT/
    rm -f OUTPUT/testmain-*.d
    echo "Please 'source BUILD' and 'run_tests'"
  else
    echo "No such component $1 found"
    exit 2
  fi
}

# If we were not sourced as a library, pass arguments onto main
if [ "$0" = "${BASH_SOURCE[0]}" ]; then
  main "$@"
else
  echo "${BASH_SOURCE[0]} sourced"
fi
