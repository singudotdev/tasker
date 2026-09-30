//! tasker-gui: tasker as a desktop app built with GPUI: buttons, dialogs and a details panel over the same
//! markdown files as the terminal UI. The tasks and everything the buttons and keys do are `tasker-core`'s;
//! this crate only draws them (`ui`). See `docs/development.md` for an overview of the modules.

// A desktop app: no console window on Windows. (Use `tasker` for the command line there.)
#![cfg_attr(windows, windows_subsystem = "windows")]

mod ui;

use tasker_core::cli::Program;

fn main() -> anyhow::Result<()> {
    Program { name: "tasker-gui", open: "open the window" }.main(std::env::args().skip(1), |app| {
        ui::run(app);
        Ok(())
    })
}
