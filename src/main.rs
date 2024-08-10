//! The HEx Helper is a cross-platform terminal UI used for modifying file data in hex or ASCII.
//! It aims to replicate some of the look of [hexyl](https://github.com/sharkdp/hexyl) while
//! functionaly acting like a terminal UI version of [GHex](https://wiki.gnome.org/Apps/Ghex).
//!
//! **heh is currently in alpha** - it's not ready to be used in any production manner. It lacks a
//! variety of quality of life features and does not store backups if killed or crashing.

use std::{error::Error, fs::OpenOptions, io, process};

use clap::{Parser, ValueEnum};
use ratatui::crossterm::tty::IsTty;

use heh::app::Application;
use heh::decoder::Encoding;

const ABOUT: &str = "
A HEx Helper to edit bytes by the nibble.

Do --help for more information.";

const LONG_ABOUT: &str = "
The HEx Helper is a terminal tool used for modifying binaries by
the nibble. It aims to replicate some of the look of hexyl while
functionally acting like a terminal UI version of GHex.

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
    CNTRLe              Switch Endianness
    CNTRLd              Page Down
    CNTRLu              Page Up
    CNTRLf or /         Search
    CNTRLn or Enter     Next Search Match
    CNTRLp              Prev Search Match

Left-clicking on a label will copy the contents to the clipboard.
Left-clicking on the ASCII or hex table will focus it.

Zooming in and out will change the size of the components.";

#[derive(Parser)]
#[command(version, about = ABOUT, long_about = LONG_ABOUT)]
struct Cli {
    #[arg(
        value_enum,
        short = 'e',
        long = "encoding",
        default_value = "ascii",
        help = "Encoding used for text editor"
    )]
    encoding: EncodingOption,
    #[arg(
        value_parser = parse_hex_or_dec,
        long = "offset",
        default_value = "0",
        help="Read file at offset (indicated by a decimal or hexadecimal number)"
    )]
    offset: usize,

    // Positional argument.
    #[arg(help = "File to open")]
    file: String,
}

/// Opens the specified file, creates a new application and runs it!
fn main() -> Result<(), Box<dyn Error>> {
    if !io::stdout().is_tty() {
        eprintln!("Stdout is not a TTY device.");
        process::exit(1);
    }

    let cli = Cli::parse();
    let file = OpenOptions::new().read(true).write(true).open(cli.file)?;
    let mut app = Application::new(file, cli.encoding.into(), cli.offset)?;
    app.run()?;

    Ok(())
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncodingOption {
    Ascii,
    Utf8,
}

impl From<EncodingOption> for Encoding {
    fn from(encoding: EncodingOption) -> Self {
        match encoding {
            EncodingOption::Ascii => Encoding::Ascii,
            EncodingOption::Utf8 => Encoding::Utf8,
        }
    }
}

fn parse_hex_or_dec(arg: &str) -> Result<usize, String> {
    if let Some(stripped) = arg.strip_prefix("0x") {
        usize::from_str_radix(stripped, 16).map_err(|e| format!("Invalid hexadecimal number: {e}"))
    } else {
        arg.parse().map_err(|e| format!("Invalid decimal number: {e}"))
    }
}
