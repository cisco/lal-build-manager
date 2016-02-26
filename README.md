# lal dependency manager
A simplified dependency manager for the Edonus code base.

## Design
`lal` is a simple command line tool that works on folders with a valid `manifest.json`, and accepts the following commands:

- `lal install` - fetch dependencies from `manifest.json` into `INPUT`
- `lal build` - build current directory's target
- `lal link` - make current `OUTPUT` available as a symlink
- `lal link dependency` - artificially substitute a dependency with a linked one
- `lal verify` - verify the dependency tree for this manifest is flat

## Manifest
Format looks like this:

```json
{
  "name": "libwebsockets",
  "scripts": {
    "build": "./BUILD libwebsockets ncp.amd64",
    "test": "./BUILD libwebsockets-unit-tests ncp.amd64"
  },
  "dependencies": {
    "ciscossl": 42
  },
  "devDependencies": {
    "gtest": 42
  }
}
```

## Updating
At some point `lal` will version check itself and let you know of a new version, and the command to update it. It can also give indications of docker container updates. This tool needs to use `npm shrinkwrap`.

## Caching
The latest `lal build` OUTPUT is available in `~/.lal/cache/NAME/local`.
Fetched versions from `lal install` is available in `~/.lal/cache/NAME/VERSION`.

## Installation
Something quick and easy. Then run `lal configure` to interactively select docker environment and default arguments to pass through to build scripts and resources. `lal configure` will create `~/.lal/lalrc`. Could also install global pre-commit hooks (e.g. to verify validity of `manifest.json`).

### Command Specification
#### lal install
lal maintains a map or a multimap `name -> (version -> prebuilt.tar.gz)` through some means. Either some web service provides the latter part of the map, through artefactory or whatnot.

If a third positional argument is given, install latest available prebuilt into `INPUT`. If `--save` is given then this also bumps the version in the manifest file.

If positional arguments are given, install all `dependencies`.
If `--dev` is given, then also install all `devDependencies`.

#### lal build
Enters docker container and run the manifest's `build` script in working directory.

#### lal link
Verifies that OUTPUT folder exists, then symlinks it to `~/.lal/cache/NAME/local`
If a component name is given as the third argument, then look for `~/.lal/cache/$3/local` and symlink that to `INPUT/NAME`.

#### lal shell
Enters an interactive shell inside the container mounting the current directory. For experimental builds with stuff like `bcm` and `opts`.

Assumes you have run `lal install` or equivalent so that `INPUT` is ready for this.

#### lal verify
Verifies that the dependency tree is flat.
Verifies that `manifest.json` is valid json.

#### lal deploy
Should be implemented so that any `deploy` scripts in the manifest file gets run.

#### lal test
Similarly if there's a `test` script in the manifest.

### Universal Options

- `--log-level=LOG_LEVEL`
- `--help` or `-h`


### Workflow
Installing and building:

```sh
lal update monolith
cd monolith
lal install --dev
# for canonical build
lal build
# for experimental
lal shell
l> ./bcm shared_tests -t
```

Updating dependencies:

```sh
lal install ciscossl [version]
lal verify # checks the tree for consistency
git commit manifest.json -m "new version of ciscossl"
git push # possibly to a branch
```

Using local versions of dependencies:

```sh
lal update ciscossl
cd ciscossl
# edit
lal build
lal link # locally 'publish' ciscossl
cd ../monolith
lal link ciscossl # link last local build of ciscssl into current INPUT
```

### Historical Documentation
Terms used herin reference [so you want to write a package manager](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527#.rlvjqxc4r) (long read).

Original [buildroot notes](https://hg.lal.cisco.com/root/files/tip/NOTES).
