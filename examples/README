## Introduction to the lal build management tool

Get hold of the `lal` tool.

    $ git clone https://github.com/lalbuild/lal; cd lal
    $ cargo build --release

Add it to your PATH

    $ cp lal/target/release/lal ~/bin/lal

Configure lal for local development

    $ lal configure ./configs/standard.json

### Running the Hello World example

The build process for a component will generate files in OUTPUT. These are packed
together and published as an ARTIFACT, available for other components to depend on.
Other components can depend on published artifacts as INPUT for their own build.

This example shows how to use `lal` as a basic dependency manager. We will create
a C static archive containing a single function, and a binary which depends on it.

Building any component is as simple as calling `lal build` in it's directory.

    $ cd examples/libhello

    $ lal build
    lal::build: Running build script in xenial container
    ...
    lal::build: Build succeeded with verified dependencies
    $ ls OUTPUT
    hello.h libhello.a lockfile.json

To create an artifact, build using the `--release` flag, and publish it.

    $ lal build --release --with-version=1 --with-sha=$(git rev-parse HEAD)
    ...
    lal::core::output: Taring OUTPUT
    $ ls ARTIFACT
    libhello.tar.gz lockfile.json
    $ lal publish libhello
    lal::publish: Publishing libhello=1 to xenial

This component is now available as a dependency to other builds.
`lal fetch` will fetch all dependencies into our build tree.

    $ cd ../hello
    $ lal fetch
    lal::fetch: Fetch xenial libhello 1
    $ ls INPUT
    libhello

With the dependencies in place, you can now build the executable.

    $ lal build
    lal::verify: Dependencies fully verified
    lal::build: Running build script in xenial container
    cc -static -o OUTPUT/hello main.c INPUT/libhello/libhello.a
    lal::build: Build succeeded with verified dependencies

Run the executable in a controlled environment

    $ lal shell ./OUTPUT/hello
    lal::shell: Entering clux/lal-xenial:latest
    Hello World!

Or execute on the host

    $ ./OUTPUT/hello
    Hello World!

### Managing versions

Let's change libhello and version bump it.

    $ cd ../libhello2
    $ lal build --release --with-version=2 --with-sha=$(git rev-parse HEAD)
    ...
    $ lal publish libhello
    lal::publish: Publishing libhello=2 to xenial

Pull in the new version of libhello.

    $ cd ../hello2
    $ lal update libhello
    lal::update: Fetch xenial libhello
    lal::storage::download: Last versions for libhello in xenial env is {1, 2}
    lal::update: Fetch xenial libhello=2

Build with the new dependencies.

    $ lal build
    lal::verify: Dependencies fully verified
    lal::build: Running build script in xenial container
    cc -static -o OUTPUT/hello main.c INPUT/libhello/libhello.a
    lal::build: Build succeeded with verified dependencies

Run it!

    $ ./OUTPUT/hello
    Hello World!
    $ ./OUTPUT/hello Ben
    Hello Ben!
