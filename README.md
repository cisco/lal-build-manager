# Langley dependency manager
[![build status](https://engci-jenkins-gpk.cisco.com/jenkins/job/team_CME/job/lal/badge/icon)](https://engci-jenkins-gpk.cisco.com/jenkins/job/team_CME/job/lal/)

A dependency manager built around artifactory and docker. See the [spec](./SPEC.md) for background information.

## Installation
Two ways to install, depending on whether you can be bothered to run the rust install script:

### Precompiled releases (instant)
Fetch the static binaries compiled with [musl](http://www.musl-libc.org/) directly from [artifactory](http://engci-maven.cisco.com/artifactory/CME-group/lal/):

```sh
curl https://engci-maven.cisco.com/artifactory/CME-group/lal/0.13.0/lal.tar | tar xz -C /usr/local
lal configure
```

Note that you will need to `sudo chown -R "$USER" /usr/local` to avoid using sudo on the tar side. Alternatively, chose your own install prefix (`-C`) and manage `$PATH` yourself.

When new verions are released, you will be told to run a similar command (but with different version numbers).

### From source (<10 minutes)
Get [stable rust](https://www.rust-lang.org/downloads.html) (inlined below), clone, build, install, and make it available:

```sh
curl -sSf https://static.rust-lang.org/rustup.sh | sh
git clone git@sqbu-github.cisco.com:Edonus/lal.git && cd lal
cargo build --release
ln -sf $PWD/target/release/lal /usr/local/bin/lal
lal configure
```

When new versions are released, you will be told to `git pull && cargo build --release`.

## Usage
Illustrated via common workflow examples below:

### Install and Update
Installing pinned versions and building:

```sh
git clone git@sqbu-github.cisco.com:Edonus/media-engine
cd edonus
lal install --dev
# for canonical build
lal build
# for experimental
lal shell
docker> ./bcm shared_tests -t
```

Updating dependencies:
(This example presumes ciscossl has independently been updated to version 6 and is ready to be used elsewhere.)

```sh
lal install ciscossl=6 --save
lal build # check it builds with new version
git commit manifest.json -m "updated ciscossl to version 6"
git push
```

### Reusing Builds
Using stashed dependencies:

```sh
git clone git@sqbu-github.cisco.com:Edonus/ciscossl
cd ciscossl
# edit
lal build
lal stash asan
cd ../monolith
lal install ciscossl=asan # install named version (always from stash)
lal build
```

This workflow replaces listing multiple components to `./build`, and `lal status` replaces the output for the build plan.

### Creating a new version
Done automatically on validated merge. Jenkins will create a tag for each successful build and that tag should be fetchable from artifactory.

### Creating a new component
Create a git repo, `lal init` it, then install deps and verify it builds.

```sh
mkdir newcomponent
cd newcomponent
lal init # create manifest
git init
git add manifest.json
git ci -m "init newcomponent"
# add git remotes (depends on where we host)
lal install gtest --save-dev
lal install libwebsockets --save
# create source and iterate until `lal build` and `lal test` succeeds
git commit -a -m "inital working version"
git push -u origin master
```

The last changeset will be tagged by jenkins if it succeeds. These have been done in two changesets here for clarity, but they could be done  in the same change.

## Developing
To hack on `lal`, follow normal install procedure, but build non-release builds iteratively.
When developing we do not do `--release`. Thus you should for convenience link `lal` via `ln -sf $PWD/target/debug/lal /usr/local/bin/lal`.

When making changes:

```sh
cargo build
lal subcommand ..args # check that your thing is good
cargo test # write tests
```

Before committing:

```sh
cargo fmt # requires `cargo install rustfmt` and $HOME/.carg/bin on $PATH
```

## Logging
Configurable via flags before the subcommand:

```sh
lal install # normal output
lal -v install # debug output
lal -vv install # all output
```

## Updating
TODO: We want an auto-update-available notification system. We also want a system to notify on new versions of the docker image.

### Influences
Terms used herein reference [so you want to write a package manager](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527#.rlvjqxc4r) (long read).

Original [buildroot notes](https://hg.lal.cisco.com/root/files/tip/NOTES).
