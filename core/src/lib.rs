//! tasker's core: tasks and their markdown files, tags, and the app's state and actions.
//! Shared by the terminal UI (`tasker`) and the desktop app (`tasker-gui`), which only draw it and
//! turn their input into [`event`] key presses, [`app::App::press`] buttons and [`app::Click`]s.
//! See `docs/development.md` for an overview of the modules.

// An internal crate, documented with `--document-private-items`: module overviews link to private modules.
#![allow(rustdoc::private_intra_doc_links)]

pub mod app;
pub mod cli;
pub mod editor;
mod enums;
pub mod event;
pub mod keys;
pub mod model;
pub mod platform;
pub mod store;
pub mod tags;
