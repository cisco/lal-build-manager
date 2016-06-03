# Edonus dependency manager [![build Status](https://engci-jenkins-gpk.cisco.com/jenkins/buildStatus/icon?job=team_CME/lal)](https://engci-jenkins-gpk.cisco.com/jenkins/job/team_CME/job/lal/)

A dependency manager built around artifactory and docker. See the [spec](./SPEC.md) for background information.

## Prerequisites
You need [docker](https://docs.docker.com/linux/step_one/) (minimum version 1.10), register an account with your username, then get someone to add the necessary credentials to your account. You will need access to the [edonusdevelopers group](https://hub.docker.com/r/edonusdevelopers/), and you need to have called `docker login` on the command line as well.

## Installation
Two ways to install, depending on whether you can be bothered to run the rust install script:

### Precompiled releases (instant)
Fetch the static binaries compiled with [musl](http://www.musl-libc.org/) directly from [artifactory](https://engci-maven.cisco.com/artifactory/CME-release/lal/):

```sh
curl https://engci-maven.cisco.com/artifactory/CME-release/lal/0.22.0/lal.tar | tar xz -C /usr/local
lal configure
```

Note that you will need to `sudo chown -R "$USER" /usr/local` to avoid using sudo on the tar side. Alternatively, chose your own install prefix (`-C`) and manage `$PATH` yourself.

When new versions are released, you will be told to run a similar command (but with different version numbers).

### From source (<10 minutes)
Get [stable rust](https://www.rust-lang.org/downloads.html) (inlined below), clone, build, install, and make it available:

```sh
curl -sSf https://static.rust-lang.org/rustup.sh | sh
git clone git@sqbu-github.cisco.com:Edonus/lal.git && cd lal
# install libssl-dev and curl (or distro equivalent) BEFORE you compile
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
lal fetch
# for canonical build
lal build
# for experimental
lal shell
docker> ./bcm shared_tests -t
```

Updating dependencies:
(This example presumes ciscossl has independently been updated to version 6 and is ready to be used elsewhere.)

```sh
lal update ciscossl=6 --save
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
lal update ciscossl=asan # update named version (always from stash)
lal build
```

This workflow replaces listing multiple components to `./build`, and `lal status` replaces the output for the build plan.

### Creating a new version
Done automatically on validated merge. Jenkins will create a tag for each successful build and that tag should be fetchable from artifactory.

### Creating a new component
Create a git repo, `lal init` it, then update deps and verify it builds.

```sh
mkdir newcomponent
cd newcomponent
lal init # create manifest
git init
git remote add origin git@sqbu-github.cisco.com:Edonus/newcomponent.git
git add manifest.json
git commit -m "init newcomponent"
# add some dependencies to manifest
lal update gtest --save-dev
lal update libwebsockets --save
# create source and iterate until `lal build`

# later..
git commit -a -m "inital working version"
git push -u origin master
```

The last changeset will be tagged by jenkins if it succeeds. These have been done in two changesets here for clarity, but they could be done  in the same change.

Note that to set up jenkins jobs and commit hooks you need to follow usage instructions on [github-config](https://sqbu-github.cisco.com/Edonus/github-config#usage), and then [jenkins-config](https://sqbu-github.cisco.com/Edonus/jenkins-config#usage).

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
cargo fmt # requires `cargo install rustfmt` and $HOME/.cargo/bin on $PATH
```

## Autocomplete
Source the completion file in your `~/.bashrc` or `~/.bash_completion`:

```sh
echo "source /usr/local/share/lal/lal.complete.sh" >> ~/.bash_completion
```

If you are installing to a different path, or compiling yourself, set the path to where you have this file. E.g., if compiling:

```sh
echo "source $PWD/lal.complete.sh" >> ~/.bash_completion
```

from source directory.

## Logging
Configurable via flags before the subcommand:

```sh
lal fetch # normal output
lal -v fetch # debug output
lal -vv fetch # all output
```

### Influences
Terms used herein reference [so you want to write a package manager](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527#.rlvjqxc4r) (long read).

Original [buildroot notes](https://hg.lal.cisco.com/root/files/tip/NOTES).
