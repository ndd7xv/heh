//! The HEx Helper is a cross-platform terminal UI used for modifying file data in hex or ASCII.
//! It aims to replicate some of the look of [hexyl](https://github.com/sharkdp/hexyl) while
//! functionaly acting like a terminal UI version of [GHex](https://wiki.gnome.org/Apps/Ghex).
//!
//! **heh is currently in alpha** - it's not ready to be used in any production manner. Notably, it
//! does not store backups if killed or crashing and there is no undo option after deleting a byte.

use std::{error::Error, fs::OpenOptions, io, process};

use clap::{arg_enum, command, value_t, Arg};
use crossterm::tty::IsTty;

use app::Application;

use crate::decoder::Encoding;

mod app;
mod character;
mod chunk;
mod decoder;
mod input;
mod label;
mod screen;
mod windows;

const ABOUT: &str = "
A HEx Helper to edit bytes by the nibble.

Do --help for more information.";

const LONG_ABOUT: &str = "
The HEx Helper is a terminal tool used for modifying binaries by
the nibble. It aims to replicate some of the look of hexyl while
functionaly acting like a terminal UI version of GHex.

Note that the octal and hexadecimal labels are slightly
different in heh; they interpret the stream as if 0s were filled
to the end of the byte (i.e. stream length 9 on FF FF would
produce octal 377 200 and hexadecimal FF 80).

Like GHex, you cannot create files with heh, only modify them.

Terminal UI Commands:
    ALT=                Increase the stream length by 1
    ALT-                Decrease the stream length by 1
    CNTRLs              Save
    CNTRLq              Quit
    CNTRLj              Jump to Byte

Left-clicking on a label will copy the contents to the clipboard.
Left-clicking on the ASCII or hex table will focus it.

Zooming in and out will change the size of the components.";

arg_enum! {
    #[derive(Copy, Clone, Debug)]
    pub enum EncodingOption {
        Ascii,
        Utf8,
    }
}

impl From<EncodingOption> for Encoding {
    fn from(encoding: EncodingOption) -> Self {
        match encoding {
            EncodingOption::Ascii => Encoding::Ascii,
            EncodingOption::Utf8 => Encoding::Utf8,
        }
    }
}

/// Opens the specified file, creates a new application and runs it!
fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .about(ABOUT)
        .long_about(LONG_ABOUT)
        .arg(
            Arg::new("Encoding")
                .help("Encoding used for text editor")
                .short('e')
                .long("encoding")
                .required(false)
                .case_insensitive(true)
                .possible_values(&EncodingOption::variants())
                .default_value("Ascii"),
        )
        .arg(Arg::new("FILE").required(true))
        .get_matches();

    if !io::stdout().is_tty() {
        eprintln!("Stdout is not a TTY device.");
        process::exit(1);
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(matches.get_one::<String>("FILE").unwrap())?;
    let encoding = value_t!(matches, "Encoding", EncodingOption)?;

    let mut app = Application::new(file, encoding.into())?;
    app.run()?;

    Ok(())
}
