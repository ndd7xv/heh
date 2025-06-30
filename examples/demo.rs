use std::path::PathBuf;

use heh::app::Application as Heh;
use heh::decoder::Encoding;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
};

fn main() {
    let path = PathBuf::from("Cargo.toml");
    let file = std::fs::OpenOptions::new().read(true).write(true).open(path).unwrap();
    let mut heh = Heh::new(file, Encoding::Ascii, 0).unwrap();

    let mut terminal = ratatui::init();
    loop {
        terminal
            .draw(|frame: &mut Frame| {
                heh.render_frame(frame, frame.area());
            })
            .expect("failed to draw frame");
        if let Event::Key(key) = event::read().expect("failed to read event") {
            if key.code == KeyCode::Char('q') {
                break;
            }
            heh.handle_input(&ratatui::crossterm::event::Event::Key(key)).unwrap();
        }
    }
    ratatui::restore();
}
