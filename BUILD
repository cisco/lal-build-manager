#!/bin/bash


run_tests() {
  # ensure we don't overwrite your config
  # back it up, then restore on EXIT
  [ -f ~/.lal/config ] && cp ~/.lal/config ./bupconfig
  restore_config() {
    [ -f bupconfig ] && mv bupconfig ~/.lal/config
  }
  trap restore_config EXIT
  ./OUTPUT/testmain-*
}

main() {
  # build in the currently available muslrust container
  set -e
  if [[ $1 == "lal" ]]; then
    mkdir -p OUTPUT/{bin,share/lal/configs}
    cp configs/* OUTPUT/share/lal/configs/
    cp lal.complete* OUTPUT/share/lal/
    if [[ $2 == "slim" ]]; then
      cargo build --no-default-features --release --verbose
      cp ./target/x86_64-unknown-linux-musl/debug/lal OUTPUT/bin/
      cp
    elif [[ $2 == "release" ]]; then
      cargo build
      cp ./target/x86_64-unknown-linux-musl/release/lal OUTPUT/bin/
    elif [[ $2 == "debug" ]]; then
      cargo build
      cp ./target/x86_64-unknown-linux-musl/debug/lal OUTPUT/bin/
    elif [[ $2 == "artifactory" ]]; then
      cargo build --release
      cp ./target/x86_64-unknown-linux-musl/release/lal OUTPUT/bin/
      echo "Please run ./package.sh if uploading to artifactory"
    else
      echo "No such configuration $2 found"
      exit 2
    fi
  elif [[ $1 == "lal-unit-tests" ]]; then
    cargo build --test testmain
    cp ./target/x86_64-unknown-linux-musl/debug/testmain-* OUTPUT/
    rm -f OUTPUT/testmain-*.d
    echo "Please backup your ~/.lal/config and run the testmain executable in OUTPUT"
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
