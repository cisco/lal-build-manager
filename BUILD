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

package_lal_tarball() {
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

create_artifact_folder() {
  # Upload to a folder on artifactory equal to the Cargo.toml version
  lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)
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
  set -e
  if [[ $1 == "lal" ]]; then
    if [[ $2 == "no-features" ]]; then
      cargo build --no-default-features --verbose
      cp ./target/x86_64-unknown-linux-musl/debug/lal OUTPUT/
    elif [[ $2 == "release" ]]; then
      cargo build --release
      cp ./target/x86_64-unknown-linux-musl/release/lal OUTPUT/
    elif [[ $2 == "debug" ]]; then
      cargo build
      cp ./target/x86_64-unknown-linux-musl/debug/lal OUTPUT/
    elif [[ $2 == "artifactory" ]]; then
      cargo build --release
      # different versioning of lal, so create ARTIFACT folder manually
      package_lal_tarball
      create_artifact_folder
      rm lal.tar
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
