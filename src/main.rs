//! The HEx Helper is a cross-platform terminal UI used for modifying file data in hex or ASCII.
//! It aims to replicate some of the look of [hexyl](https://github.com/sharkdp/hexyl) while
//! functionaly acting like a terminal UI version of [GHex](https://wiki.gnome.org/Apps/Ghex).
//!
//! **heh is currently in alpha** - it's not ready to be used in any production manner. Notably, it
//! does not store backups if killed or crashing and there is no undo option after deleting a byte.

use std::{error::Error, fs::OpenOptions, io, process};

use crossterm::tty::IsTty;

use app::Application;

mod app;
mod byte;
mod input;
mod label;
mod screen;
mod windows;

use clap::{arg, command};

const ABOUT: &str = "
A HEx Helper to edit bytes by the nibble.

Do --help for more information.";

const LONG_ABOUT: &str = "
The HEx Helper is a terminal tool used for modifying binaries by the nibble.
It aims to replicate some of the look of hexyl while functionaly acting like
a terminal UI version of GHex.

Note that the octal and hexadecimal labels are slightly different in heh; they
interpret the stream as if 0s were filled to the end of the byte (i.e. stream
length 9 on FF FF would produce octal 377 200 and hexadecimal FF 80).

Like GHex, you cannot create files with heh, only modify them.

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

Zooming in and out will change the size of the components.";

/// Opens the specified file, creates a new application and runs it!
fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .about(ABOUT)
        .long_about(LONG_ABOUT)
        .arg(arg!([FILE]).required(true))
        .get_matches();

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(matches.get_one::<String>("FILE").unwrap())?;

    if !io::stdout().is_tty() {
        eprintln!("Stdout is not a TTY device.");
        process::exit(1);
    }

    let mut app = Application::new(file)?;
    app.run()?;

    Ok(())
}
