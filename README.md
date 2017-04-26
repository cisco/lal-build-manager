# lal dependency manager [![build Status](https://engci-jenkins-gpk.cisco.com/jenkins/buildStatus/icon?job=team_CME/lal)](https://engci-jenkins-gpk.cisco.com/jenkins/job/team_CME/job/lal/)

A dependency manager built around artifactory and docker. See the [spec](./SPEC.md) for background information.

## Prerequisites
You need [docker](https://docs.docker.com/engine/installation/linux/) (minimum version 1.10), register an account with your username, then get someone to add the necessary credentials to your account on dockerhub.

In particular, your account will need access to the images in the [relevant config file](https://sqbu-github.cisco.com/Edonus/lal/tree/master/configs), and you need to have called `docker login` on the command line with this account.

## Installation
Two ways to install, depending on whether you can be bothered to run the rust install script:

### Precompiled releases (instant)
Fetch the static binaries compiled with [musl](http://www.musl-libc.org/) directly from [artifactory](https://engci-maven.cisco.com/artifactory/CME-group/lal/):

```sh
curl https://engci-maven.cisco.com/artifactory/CME-group/lal/latest/lal.tar | tar xz -C /usr/local
echo "source /usr/local/share/lal/lal.complete.sh" >> ~/.bash_completion
source ~/.bash_completion # or open new shell
lal configure <site-config> # use autocomplete to select config
```

Note that **you will need** to `sudo chown -R "$USER" /usr/local` first *if* you want to use this as the install prefix because automatic upgrades will happen inside that folder. Alternatively, install to another location and manage `$PATH` yourself.

There will be a daily upgrade attempt that auto-upgrades your version if a new one was found.

### From source (<10 minutes)
Get [stable rust](https://www.rust-lang.org/downloads.html) (inlined below), clone, build, install, and make it available:

```sh
curl https://sh.rustup.rs -sSf | sh
# `rustup update stable` - to upgrade rust later
git clone git@sqbu-github.cisco.com:Edonus/lal.git && cd lal
cargo build --release
ln -sf $PWD/target/release/lal /usr/local/bin/lal
echo "source $PWD/lal.complete.sh" >> ~/.bash_completion
source ~/.bash_completion # or open new shell
lal configure <site-config> # use autocomplete to select config
```

When new versions are released, you will be told to `git pull && cargo build --release`.

## Usage
Illustrated via common workflow examples below:

### Install and Update
Installing pinned versions and building:

```sh
git clone git@sqbu-github.cisco.com:Edonus/media-engine
cd media-engine
lal fetch
# for canonical build
lal build
# for experimental
lal shell
docker> ./local_script
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
cd ../media-engine
lal update ciscossl=asan # update named version (always from stash)
lal build -s
```

This workflow allows building multiple components simultaneously, and `lal status` provides safeguards and information on what dependencies you are using. Note that while doing this, you will receive warnings that you are using non-canonical dependencies.

### Creating a new version
Designed to be handled by jenkins on each push to master (ideally through validated merge). Jenkins should create your numeric tag and upload the build output to artifactory. This behaviour is handled in [jenkins-config](https://sqbu-github.cisco.com/Edonus/jenkins-config).

### Creating a new component
Create a git repo, `lal init` it, then update deps and verify it builds.

```sh
mkdir newcomponent
cd newcomponent
lal init xenial # create manifest for a xenial component
git init
git remote add origin git@sqbu-github.cisco.com:Edonus/newcomponent.git
git add manifest.json
git commit -m "init newcomponent"
# add some dependencies to manifest
lal update gtest --save-dev
lal update libwebsockets --save
# create source and iterate until `lal build` passes

# later..
git commit -a -m "inital working version"
git push -u origin master
```

The last changeset will be tagged by jenkins if it succeeds. These have been done in two changesets here for clarity, but they could be done  in the same change.

Note that to set up jenkins jobs and commit hooks you need to follow usage instructions on [github-config](https://sqbu-github.cisco.com/Edonus/github-config#usage), and then [jenkins-config](https://sqbu-github.cisco.com/Edonus/jenkins-config#usage).

## Docker Image
The `build` and `shell` commands will use `docker run` on a configured image. For this to work without messing with permissions, two conditions must be met:

- configured docker image must have a `lal` user with uid `1000`
- linux user outside docker must be have uid `1000`

We have found this can be satisfied for most linux users and owned containers. The linux user restriction is unfortunately easier than to get docker usernamespaces working (which is currently incompatible with features like host networking).

## Developing
To hack on `lal`, follow normal install procedure, but build non-release builds iteratively.
When developing we do not do `--release`. Thus you should for convenience link `lal` via `ln -sf $PWD/target/debug/lal /usr/local/bin/lal`.

When making changes:

```sh
cargo build
lal subcommand ..args # check that your thing is good
cargo test # write tests
```

Good practices before comitting (not mandatory):

```sh
cargo fmt # requires `cargo install rustfmt` and $HOME/.cargo/bin on $PATH
rustup run nighthly cargo clippy # requires rustup.rs install of rust + nightly install of clippy
```

## Build issues
If libraries cannot be built, then upgrade `rustc` by running `rustup update stable`.

## Logging
Configurable via flags before the subcommand:

```sh
lal fetch # normal output
lal -v fetch # debug output
lal -vv fetch # all output
```

### Influences
Terms used herein reference [so you want to write a package manager](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527#.rlvjqxc4r) (long read).
