#!/bin/bash
set -ex

cargo doc
echo "<meta http-equiv=refresh content=0;url=lal/index.html>" > target/doc/index.html
ghp-import -n target/doc
git push -qf "git@github.com:lalbuild/lal.git" gh-pages
