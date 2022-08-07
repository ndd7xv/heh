use std::error::Error;
use std::fs::OpenOptions;
use std::process;

use app::Application;

mod app;
mod byte;
mod label;
mod screen;

use clap::{arg, command};

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

Terminal UI Commands:
    =                   Increase the stream length by 1
    -                   Decrease the stream length by 1
    CNTRLs              Save
    CNTRLq              Quit

Left-clicking on a label will copy the contents to the clipboard.
Left-clicking on the ASCII or hex table will focus it.

Zooming in and out will change the size of the components.";

fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .about(ABOUT)
        .long_about(LONG_ABOUT)
        .arg(arg!([FILE]).required(true))
        .get_matches();

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(matches.get_one::<String>("FILE").unwrap())
        .unwrap_or_else(|err| {
            eprintln!("An error occured opening the file: {err:?}");
            process::exit(1)
        });

    let mut app = Application::new(file)?;
    app.run()?;

    Ok(())
}
