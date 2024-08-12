# heh

[![Crates.io](https://img.shields.io/crates/v/heh.svg)](https://crates.io/crates/heh)
[![Codecov](https://codecov.io/github/ndd7xv/heh/coverage.svg?branch=master)](https://codecov.io/gh/ndd7xv/heh)
[![Dependency status](https://deps.rs/repo/github/ndd7xv/heh/status.svg)](https://deps.rs/repo/github/ndd7xv/heh)
<!--- [![Documentation](https://docs.rs/heh/badge.svg)](https://docs.rs/heh/) If https://github.com/rust-lang/docs.rs/issues/238#issuecomment-631333050 is ever closed?-->

The HEx Helper is a cross-platform terminal [hex editor](https://en.wikipedia.org/wiki/Hex_editor) used for modifying file data in hex or ASCII. It aims to replicate some of the look of hexyl while functionally acting like a terminal UI version of GHex.

> [!WARNING]
> heh is currently in alpha - it's not ready to be used in any production manner. It lacks a variety of quality of life features and does not store backups if killed or crashing.

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

## Using as a Ratatui widget

`heh` can be used a library and embedded into other TUI applications which use [Ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

Add `heh` to your dependencies in `Cargo.toml`:

```toml
[dependencies]
ratatui = "0.24"
crossterm = "0.27"
heh = "0.4"
```

Create the application:

```rust
use heh::app::Application as Heh;
use heh::decoder::Encoding;

let file = std::fs::OpenOptions::new().read(true).write(true).open(path).unwrap();
let heh = Heh::new(file, Encoding::Ascii, 0).unwrap();
```

Then you can render a frame as follows:

```rust
terminal.draw(|frame| {
    heh.render_frame(frame, frame.size());
});
```

To handle key events:

```rust
heh.handle_input(&ratatui::crossterm::event::Event::Key(/* */)).unwrap();
```

See the [binsider](https://github.com/orhun/binsider) project for an example use case.

# Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
