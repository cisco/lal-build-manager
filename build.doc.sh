#!/bin/bash
# NB: requires the ghp-import pip module
cargo doc
echo "<meta http-equiv=refresh content=0;url=lal/index.html>" > target/doc/index.html
ghp-import -n target/doc
git push -qf "git@sqbu-github.cisco.com:Edonus/lal.git" gh-pages

# NB: to iterate locally:
# cargo doc && xdg-open target/doc/lal/index.html
