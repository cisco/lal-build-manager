#!/bin/bash

# THIS SCRIPT SHOULD BE INVOKED BY CI INSTEAD OF lal publish
# This is because lal intentionally uses semver.


# If you have done `lal build lal --release`
# It will convert the standard release structure of ARTIFACT:
#
# ARTIFACT/
# ├── lal.tar.gz
# └── lockfile.json
#
# And converts it into this folder structure:
#
# ARTIFACT/
# ├── 3.3.3
# │   └── lal.tar.gz
# ├── latest
# │   └── lal.tar.gz
# └── lockfile.json
#
# Such that jenkins will upload this verbatim to:
# http://engci-maven.cisco.com/artifactory/CME-release/lal/
# This is the canonical source of lal AT THE MOMENT
# It is also hardcoded in `lal upgrade` so this works.
# We cannot (and should never) `lal publish lal` because lal uses semver.
# We also would not want it accidentally introduced into the normal dependency tree.
#
# crates.io may become the canonical one in the future if it is open sourced.

mutate_artifact_folder() {
  local -r lalversion=$(grep version Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)
  # Guard on version not existing
  buildurl="http://engci-maven.cisco.com/artifactory/api/storage/CME-release/lal"
  if curl -s "${buildurl}" | grep -q "$lalversion"; then
      echo "lal version already uploaded - stopping" # don't want to overwrite
      # don't want to upload anything accidentally - jenkins is dumb
      rm -rf ARTIFACT/
  else
    echo "Packaging new lal version"
    mkdir "ARTIFACT/${lalversion}" -p
    mv ARTIFACT/lal.tar.gz "ARTIFACT/${lalversion}/lal.tar.gz"
    # Overwrite the latest folder
    cp "ARTIFACT/${lalversion}" "ARTIFACT/latest" -R
  fi
}


main() {
  set -e
  if [ ! -f ARTIFACT/lal.tar.gz ]; then
    echo "No release build of lal found"
    rm -rf ARTIFACT # just in case
    exit 2
  fi
  echo "Found release build with:"
  tar tvf ARTIFACT/lal.tar.gz
  mutate_artifact_folder
}

main "$@"
