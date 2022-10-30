# heh

The HEx Helper is a cross-platform terminal UI used for modifying file data in hex or ASCII. It aims to replicate some of the look of hexyl while functionaly acting like a terminal UI version of GHex.

__heh is currently in alpha - it's not ready to be used in any production manner. Notably, it does not store backups if killed or crashing.__

![screenshot of heh](demo.png)

# Installation and Usage

heh is currently only available via cargo:

```
cargo install heh
```

From `heh --help`:
```
...
Terminal UI Commands:
    ALT =               Increase the stream length by 1
    ALT -               Decrease the stream length by 1
    CTRL c              Copy Selection in Current Editor (Hex or ASCII)
    CTRL ALT c          Copy Selection in Other Editor
    CTRL j              Jump to Byte
    CTRL q              Quit
    CTRL s              Save
    CTRL z              Undo

Left-clicking on a label will copy the contents to the clipboard.
Left-clicking on the ASCII or hex table will focus it. Dragging the click will
select the specified text.

Zooming in and out will change the size of the components.

USAGE:
    heh <FILE>

ARGS:
    <FILE>
...

```
# Contributing

Thanks for you interest in contributing! Changes of all types are welcome, from feature additions and major code refactoring to the tiniest typo fix. If you want to make change,

1. Fork this repository
2. Make desired changes in (a) descriptive commit(s)
3. Make a PR, linking any issues that may be related

...and follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct) all the way through!