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
curl -sSL https://engci-maven.cisco.com/artifactory/CME-group/lal/3.4.2/lal.tar | tar xz -C /usr/local
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
# install libssl-dev and curl (or distro equivalent) + `cargo clean` if build fails
cargo build --release
ln -sf $PWD/target/release/lal /usr/local/bin/lal
echo "source $PWD/lal.complete.sh" >> ~/.bash_completion
source ~/.bash_completion # or open new shell
lal configure <site-config> # use autocomplete to select config
```

When new versions are released, you will be told to `git pull && cargo build --release`.

### With lal (future upgrades)
This will upgrade an installation done from an artifactory download (as an original boostrap), and it will upgrade any future upgrade that used this setup.

```sh
git clone git@sqbu-github.cisco.com:Edonus/lal.git && cd lal
lal build --release
tar xzf ARTIFACT/lal.tar.gz -C /usr/local
```

You can also build a slim version of lal without autoupgrade this way (because you are upgrading yourself). Just replace the build with `lal build -c slim --release` above.

Note that if you configured autocomplete, it is still configured.

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
The `build` and `shell` commands will use `docker run` on a configured image. The only condition we require of docker images is that they have a `lal` user added.

We will use this user inside the container to run build scripts. By default this works best if the `id` of the host user is 1000:1000, but if it is not, then lal will create a slightly modified version of the image that matches the user id and group id for your host system.

This is a one time operation, and it is a more general solution for use than docker usernamespaces (which is currently incompatible with features like host networking).

## Developing
To hack on `lal`, follow normal install procedure, but build non-release builds iteratively.
When developing we do not do `--release`. Thus you should for convenience link `lal` via `ln -sf $PWD/target/debug/lal /usr/local/bin/lal`.

When making changes:

```sh
cargo build
./target/debug/lal subcommand ..args # check that your thing is good
cargo test # write tests
```

Note that the tests overwrite your `~/.lal/config` so you may find the `run_tests` function in `BUILD` useful. You can alternatively:

```sh
source BUILD
lal build lal-unit-tests && run_tests
```

We can't run the `lal-unit-tests` with `lal` because the test executable invokes `docker`.

Good practices before comitting (not mandatory):

```sh
cargo fmt # requires `cargo install rustfmt` and $HOME/.cargo/bin on $PATH
rustup run nighthly cargo clippy # requires rustup.rs install of rust + nightly install of clippy
```

## Build issues
If libraries cannot be built, then upgrade `rustc` by running `rustup update stable`.

- fatal error: 'openssl/hmac.h' file not found If you are on a GNU/Linux distribution (like Ubuntu), please install `libssl-dev`. If you are on OSX, please install openssl and check your OpenSSL configuration:

```sh
brew install openssl
export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
export DEP_OPENSSL_INCLUDE=`brew --prefix openssl`/include # should work without this
```

There's also a runtime lookup of certificates to do peer verification of certificates. This requires having set:

```
# OSX
export SSL_CERT_FILE=/usr/local/etc/openssl/cert.pem
# CentOS
export SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt
```

This should be put in your `~/.bashrc` or `~/.bash_profile` as `lal` reads it on every run. Note that the default location is `/etc/ssl/certs/ca-certificates.crt` and that is correct for most linux distros.

## Logging
Configurable via flags before the subcommand:

```sh
lal fetch # normal output
lal -v fetch # debug output
lal -vv fetch # all output
```

### Influences
Terms used herein reference [so you want to write a package manager](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527#.rlvjqxc4r) (long read).
