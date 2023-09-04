# heh

[![Crates.io](https://img.shields.io/crates/v/heh.svg)](https://crates.io/crates/heh)
[![Codecov](https://codecov.io/github/ndd7xv/heh/coverage.svg?branch=master)](https://codecov.io/gh/ndd7xv/heh)
[![Dependency status](https://deps.rs/repo/github/ndd7xv/heh/status.svg)](https://deps.rs/repo/github/ndd7xv/heh)
<!--- [![Documentation](https://docs.rs/heh/badge.svg)](https://docs.rs/heh/) If https://github.com/rust-lang/docs.rs/issues/238#issuecomment-631333050 is ever closed?-->

The HEx Helper is a cross-platform terminal UI used for modifying file data in hex or ASCII. It aims to replicate some of the look of hexyl while functionally acting like a terminal UI version of GHex.

__heh is currently in alpha - it's not ready to be used in any production manner. Notably, it does not store backups if killed or crashing and there is no undo option after deleting a byte.__

![screenshot of heh](demo.png)

# Installation and Usage

heh is available via cargo:

```
cargo install heh
```

From `heh --help`:
```
...
Terminal UI Commands:
    ALT=                Increase the stream length by 1
    ALT-                Decrease the stream length by 1
    CNTRLs              Save
    CNTRLq              Quit
    CNTRLj              Jump to Byte
    CNTRLe              Change endianness
    CNTRLd              Page Down
    CNTRLu              Page Up
    CNTRLf or /         Search
    CNTRLn or Enter     Next Search Match
    CNTRLp              Prev Search Match

Left-clicking on a label will copy the contents to the clipboard.
Left-clicking on the ASCII or hex table will focus it.

Zooming in and out will change the size of the components.

USAGE:
    heh <FILE>

ARGS:
    <FILE>
            

OPTIONS:
    -h, --help
            Print help information

    -V, --version
            Print version information

```

## Distro packages

<details>
  <summary>Packaging status</summary>

[![Packaging status](https://repology.org/badge/vertical-allrepos/heh.svg)](https://repology.org/project/heh/versions)

</details>

If your distribution has packaged `heh`, you can use that package for the installation.

### Arch Linux

You can use [pacman](https://wiki.archlinux.org/title/Pacman) to install from the [extra repository](https://archlinux.org/packages/extra/x86_64/heh/):

```
pacman -S heh
```

### Alpine Linux

`heh` is available for [Alpine Edge](https://pkgs.alpinelinux.org/packages?name=heh&branch=edge). It can be installed via [apk](https://wiki.alpinelinux.org/wiki/Alpine_Package_Keeper) after enabling the [testing repository](https://wiki.alpinelinux.org/wiki/Repositories).

```
apk add heh
```

# Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
