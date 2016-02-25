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
  "build": "./BUILD libwebsockets ncp.amd64",
  "dependencies": {
    "ciscossl": "SEMVER||NUMBER"
  }
}
```

## Updating
At some point `lal` will version check itself and let you know of a new version, and the command to update it. It can also give indications of docker container updates.

## Caching
The latest `lal build` OUTPUT is available in `~/.lal/NAME/local`.
Fetched versions from `lal install` is available in `~/.lal/NAME/VERSION`.

## Installation
Comes bundled with the container. You mount `~/.lal`?

### Command Specification
#### lal install
lal maintains a map or a multimap `name -> (version -> prebuilt.tar.gz)` through some means. Either some web service provides the latter part of the map, through artefactory or whatnot.

#### lal build
Enters docker container and run the manifest's `build` script in working directory.

#### lal link
Verifies that OUTPUT folder exists, then symlinks it to `~/.lal/NAME/local`
If a component name is given as the third argument, then look for `~/.lal/$3/local` and symlink that to `INPUT/NAME`.

#### lal shell
Enters an interactive shell inside the container mounting the current directory. For experimental builds with stuff like `bcm` and `opts`.

Assumes you have run `lal install` or equivalent so that `INPUT` is ready for this.
