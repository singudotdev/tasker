//! tasker: a terminal todo tracker (todo → doing → done), backed by markdown files.
//! The tasks, their files and everything the keys do are `tasker-core`'s; this crate is the terminal:
//! `tui` runs the session and turns crossterm input into the core's, and `ui` draws the screens.
//! See `docs/development.md` for an overview of the modules.

mod tui;
mod ui;

use tasker_core::cli::Program;

fn main() -> anyhow::Result<()> {
    Program { name: "tasker", open: "open the TUI" }.main(std::env::args().skip(1), tui::run)
}
