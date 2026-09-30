# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-09-30

First release.

### Added

- Terminal UI to create, start, finish, edit and delete tasks, in the "lazy" style of lazygit and lazyssh:
  a task list with a details panel (status and since when, recent history, description, latest comment), a footer that always shows the keys in mnemonic style (`(n)ew  (e)dit  (x) delete`),
  a runnable `?` keys menu, and mouse support (wheel, click).
- Four statuses, `todo`, `doing`, `parked` (started, set aside for now) and `done`, one key each
  (`s` start, `p` park, `d` done, `b` back to todo), from the list or the task view.
  Any number of tasks can be in doing. A header counts the tasks in each status.
- Status history: every change is recorded with its date and time, shown in the task view, the details panel,
  and as a "since" column in the list.
- Tasks work like issues: a description (`m`) and timestamped comments that can be added (`c`), edited and deleted,
  shown in a task view (`v`). Comments record when they were edited.
- In-place multi-line text editor for descriptions and comments: `esc` saves, `ctrl+c` discards (in the terminal).
- One-line fields (title, tags, tag rename, search) have a movable cursor: `←` `→` `home` `end`, `backspace` and `delete`.
- Every key that works is shown in the footer; there are no hidden aliases.
- Tags with persistent colors that don't repeat until the palette is used up, customizable in `tags.md`.
- Tags screen (`t`): every tag with its color and how many tasks in each status use it. Rename a tag on every task (`r`;
  renaming to an existing tag merges the two), change its color (`c` / `C`), or delete it from every task (`x`).
- Live search with `/` over titles and tags (`#tag` matches tags only).
- Task list views: active, all, or done only (`f`).
- **Desktop app, `tasker-gui`**, on the same tasks and files as the terminal UI.
  A conventional window: a toolbar with New task, search and Active / All / Done filters; the task list with a details panel
  holding status buttons (To do, Doing, Parked, Done) and Open, Edit, Open file and Delete; the task view with Edit and
  Delete on every comment; the tags screen with color, rename and delete buttons; dialogs with Cancel and Save.
  Double-click opens a task. The terminal's keys work too: every button's tooltip names its key, and `?` lists them.
- Desktop text fields and editor support the clipboard: `Ctrl+A` selects all, `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copy, cut
  and paste (`Cmd` on macOS), and `Shift` with the arrows, `Home` or `End` selects. Typing or pasting replaces the selection.
- CLI: `tasker list [doing|todo|parked|done|all] [--tag TAG]` prints tasks as markdown; `tasker path`, `tasker --version`.
- One-line installers: `install.sh` (Linux, macOS; curl or wget) and `install.ps1` (Windows; `irm | iex`),
  with checksum verification, no admin rights needed, PATH setup and version pinning. They install either app, or both:
  `--app tui|gui|both` (`$env:TASKER_APP` on Windows), the terminal UI by default. The desktop app gets an application
  menu entry on Linux and a Start Menu shortcut on Windows.
- Per-user only: installers refuse root/sudo and elevated PowerShell and only install inside the user's home;
  the app refuses to run as root (opt-in for root-only containers: `TASKER_ALLOW_ROOT=1`).
- Uninstall (program only) and purge (program + tasks and tags), with confirmation, which only deletes files tasker created.
- Released under the MIT license, with CI (lint and tests on Linux, macOS and Windows) and automated release builds.
- Plain markdown storage, one file per task, with atomic writes; hand edits are picked up with `R` or after `o`.
- A Cargo workspace: `core/` (shared logic), `tui/` (`tasker`) and `gui/` (`tasker-gui`).
