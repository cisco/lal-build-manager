#!/bin/bash
set -ex

ver=$(grep version Cargo.toml | head -n 1 | awk -F'"' '{print $2}')

git tag -a "v${ver}" -m "${ver}"
