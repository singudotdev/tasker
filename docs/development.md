# Development

## Building and testing

The repository is a Cargo workspace with three crates:

| crate | folder | what |
| --- | --- | --- |
| `tasker-core` | `core/` | everything both apps share: tasks, files, tags, the app's state and actions, key bindings, the command line |
| `tasker` | `tui/` | the terminal UI (ratatui) |
| `tasker-gui` | `gui/` | the desktop app (GPUI) |

Plain `cargo` commands cover the core and the terminal UI (the workspace's default members), which build fast
and without GPUI. Add `-p tasker-gui` or `--workspace` for the desktop app.

```sh
cargo run                                  # run the TUI (debug build)
cargo run -- list all                      # run a subcommand
cargo run -p tasker-gui                    # run the desktop app
cargo test                                 # core + TUI tests
cargo test --workspace                     # … and the desktop app's
cargo clippy --workspace --all-targets     # lints: must be warning-free (pedantic is on, see below)
cargo fmt --all                            # formatting (rustfmt.toml)
cargo build --release --workspace          # optimized, stripped binaries in target/release/
```

The desktop app needs its graphics libraries' development packages on Linux, see
[Build from source](getting-started.md#requirements).

tasker refuses to run as root. In containers where you are root, add `-e TASKER_ALLOW_ROOT=1` (tests don't need it).

Use a throwaway data folder while developing so you don't touch your real tasks:

```sh
TASKER_DIR=/tmp/tasker-dev cargo run
TASKER_DIR=/tmp/tasker-dev cargo run -p tasker-gui
```

No local Rust toolchain? Build in a container instead:

```sh
podman run --rm -v "$PWD":/src:Z -w /src docker.io/library/rust:1 cargo test
```

## Code style

- **Formatting:** `cargo fmt`, configured in `rustfmt.toml` (120 columns; short constructs stay on one line).
- **Lints:** `Cargo.toml` enables clippy's `pedantic` group for the whole crate and denies `unsafe` code.
  The few allowed exceptions are listed there with their reason. The only `unsafe` block is the root check in `platform.rs`.
- **Small modules with one job**, each starting with a `//!` comment that says what it's for. Functions stay short;
  when a `match` arm grows, it becomes a named helper (`key_on_task`, `add_comment`, …).
- **Comments:** every item (type, field, variant, function, constant) has a `///` doc comment saying what it's for,
  and every file starts with a `//!` comment describing the module. Inside functions, comments explain *why* (or the
  non-obvious *what*), not what each line already says. Check for undocumented items and broken doc references with:

  ```sh
  cargo clippy --workspace -- -W clippy::missing_docs_in_private_items
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items   # also browsable: target/doc/tasker_core/, target/doc/tasker/, target/doc/tasker_gui/
  ```
- **Fixed lists of values are declared once.** Statuses and the done-task views
  are declared with the macros in `enums.rs`: one line per variant, `Variant => "name"`. The list of all variants
  (`ALL`), their order (`position`, `next`), and the conversions to and from text (`as_str`, `parse`) are generated
  from that line, so they can't get out of sync. The `--help` text is built from the same names.
  For example, adding a status is one line in `model/mod.rs`; the header, the tags screen, `tasker list` and its
  help text pick it up. Declaration order matters: statuses are declared in the order the task list shows them,
  and the done-task views in the order `f` cycles through them.
- **Named constants** instead of magic numbers (`MESSAGE_TTL`, `RECENT_CHANGES`, layout thresholds, …).
- **Errors:** `anyhow::Result` with context (`writing <path>`) at the edges. Errors while the TUI runs become a footer message,
  never a crash. `expect` is only used for things that cannot fail (writing to a `String`, calendar arithmetic),
  and the message says why.
- **Every key that works is listed** in `keys.rs` and therefore shown in the footer. The test `unlisted_keys_do_nothing`
  and `every_listed_list_key_does_something` guard this.

## Dependencies

The crates use the **Rust 2024 edition**. Shared versions and lints are in the workspace `Cargo.toml`.

| crate | version | used for |
| --- | --- | --- |
| [`ratatui`](https://ratatui.rs) | 0.30 | `tui`: terminal UI, with its re-exported `crossterm` backend. Default features are off; only `crossterm` and `layout-cache` are enabled |
| [`gpui`](https://www.gpui.rs) | Zed 1.21.0 (git) | `gui`: the desktop app's window, layout and input. From Zed's repository, pinned to a tag: the crates.io release (0.2.2) can't draw on NVIDIA with Wayland, and newer GPUI isn't published. The workspace `Cargo.toml` carries the `[patch]` entries Zed builds it with |
| [`chrono`](https://docs.rs/chrono) | 0.4 | dates, times, time zones |
| [`dirs`](https://docs.rs/dirs) | 7 | platform data folder |
| [`anyhow`](https://docs.rs/anyhow) | 1 | error handling |
| [`tempfile`](https://docs.rs/tempfile) | 3 | tests only: temporary data folders |

The minimum supported Rust version of the core and the terminal UI is **1.88** (`rust-version` in `core/Cargo.toml` and
`tui/Cargo.toml`): the code uses let chains (`if a && let Some(b) = c`), which edition 2024 allows from Rust 1.88. Cargo picks
dependency versions that still support it. CI checks it with Rust 1.88, and it's fine to develop on any newer Rust.
The desktop app follows GPUI, which needs a recent stable Rust.
Keep `Cargo.lock` committed: tasker is an application, and the lockfile makes builds reproducible.

To update dependencies: `cargo update` for compatible releases; for new major versions, change the version in
`Cargo.toml`, then run `cargo build` and the full check (`cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) and try both apps.
To move GPUI to a newer Zed release, change the tag in `gui/Cargo.toml` and copy that release's `[patch.crates-io]`
entries from Zed's workspace `Cargo.toml`.

## Architecture

```
core/src/            tasker-core: shared by both apps
├── lib.rs
├── cli.rs           Program (the binary's name), Command / ListArgs parsing, `list`, `path`
├── platform.rs      OS specifics: the per-user (not root) check
├── event.rs         KeyCode / KeyEvent / KeyModifiers: the input both apps translate theirs into
├── app/             state and behaviour
│   ├── mod.rs       App: tasks, selection, status message, loading, queries
│   ├── mode.rs      the screens and dialogs (Mode) and the state each needs
│   ├── actions.rs   operations: status changes, create, delete, descriptions, comments, tags
│   └── input.rs     keys → actions; the wheel; buttons (`press`) and other clicks (`Click`); select all, copy, cut, paste; the mouse in text (`point`)
├── model/           the domain
│   ├── mod.rs       Task, Status, StatusChange, Comment
│   ├── markdown.rs  the task file format: writing and parsing
│   ├── text.rs      tags, slugs, heading rules for user text
│   └── time.rs      timestamps and ages (`5m`, `3h`, `2d`)
├── enums.rs         `named_enum!` / `ordered_enum!`: enums whose variants are listed in one place
├── keys.rs          key bindings per screen: the source for the TUI's footer and both apps' `?` menus
├── editor.rs        text editing with a selection: multi-line TextEditor (descriptions, comments), one-line LineInput (form, rename, search)
├── store.rs         the data folder: task files, tags.md
└── tags.rs          tag colors: palette, no-repeat assignment, tags.md format

tui/src/             tasker: the terminal UI
├── main.rs          entry point: the command line, opening the TUI
├── tui.rs           terminal setup and teardown, panic hook, event loop (crossterm → core input), $EDITOR handoff
└── ui/              drawing only: reads App, renders with ratatui
    ├── mod.rs       draw() dispatch, ListArea (where the list is, for clicks), tag chips, status icons, popup, wrap
    ├── hints.rs     mnemonic key hints and the footer
    ├── list.rs      header, task list, details panel, search bar
    ├── issue.rs     task view (description, history, comments)
    ├── tags.rs      tags screen (rename, recolor, delete)
    └── dialogs.rs   form, delete confirmations, keys menu, text editor, rename tag

gui/src/             tasker-gui: the desktop app
├── main.rs          entry point: the command line, opening the window
└── ui/              the window: draws App with GPUI and turns clicks and keys into the core's input
    ├── mod.rs       the root view, status bar, buttons that run an action (`action`), GPUI keystrokes → core keys, clipboard shortcuts
    ├── widgets.rs   buttons, segmented controls, text fields, status pills, tag chips, cards, tooltips
    ├── pointer.rs   the mouse in text fields and the editor: place the cursor, drag to select, double/triple-click
    ├── caret.rs     the text cursor: a thin line drawn over the text, blinking (`Blink`, restarted on every input)
    ├── menu.rs      right-click menus for tasks, comments, tags and text
    ├── theme.rs     colors, fonts, and styled text (`Line` / `Span`)
    ├── list.rs      toolbar (new, search, filters), task table, details panel
    ├── issue.rs     task view: status, description, history, comment cards
    ├── tags.rs      tags screen with color, rename and delete buttons
    └── dialogs.rs   form, delete confirmations, keyboard shortcuts, text editor, rename tag

core/tests/, tui/tests/, gui/tests/   each crate's tests, laid out like its src/ (see Tests)
```

### Data flow

```
key press ─────────────────▶ App::on_key (input.rs) ──▶ action (actions.rs)
button (desktop) ──▶ App::press ──┘
                                             │
                                             ├─▶ Task::set_status (model: records the change in history)
                                             └─▶ Store::save  ──▶  NNNN-slug.md (atomic write)

every 500 ms / after each input ──▶ each app's ui draws &App   ("since" ages are computed from `now`)
```

- **The files are the source of truth.** Every change is written immediately; there's no in-memory state that could be lost.
  `App::reload` rebuilds everything from disk. It runs after `$EDITOR` returns and on `R`.
- **Status changes** go through `Task::set_status`, which appends a `StatusChange` to the history only when the status
  really changes. The `- status:` line in the file is the truth; the history is the record of how the task got there.
- **Modes** (`app::Mode`): `Normal`, `Input(Form)`, `Search`, `Confirm(Confirm)`, `Keys(KeysMenu)`,
  `Issue(IssueView)` (task view), `Edit(Edit)` (text editor with a `Target`: description, new comment
  or existing comment, and the view to return to), `Tags(TagsView)` and `RenameTag(RenameTag)`. `on_key` takes the current mode out, handles the key, and each handler
  puts back the mode to continue in.
- **Keys menu:** `?` lists the screen's bindings from `keys.rs`. Running one replays its key through `App::on_key`,
  so the menu can never do something the key itself wouldn't.
- **One behaviour, two apps.** Both apps translate their input into the core's: key presses into `event::KeyEvent`
  for `App::on_key`, and clicks into `App::click` (select a row, a comment or a tag, focus a field or the search box,
  pick a filter). A desktop button runs its action's key with `App::press`, which leaves the search box first so the
  key isn't typed into it. So a button can never do something its key wouldn't, and the behaviour tests cover both apps.
- **Clipboard (desktop only):** the terminal owns copy and paste there, so these aren't keys. `ui::TaskerView::clipboard`
  catches `ctrl+a/c/x/v` (`cmd` on macOS) before `App::on_key` and calls `App::select_all`, `selected_text`, `cut` and
  `paste` with the system clipboard. The selection itself lives in the core editors: `shift` with a movement key
  (`KeyModifiers::SHIFT`, which only the desktop app sends) extends it, and typing or pasting replaces it.
- **Mouse in the terminal:** `App::wheel` turns the wheel into `↑`/`↓`, so every screen scrolls the way its arrow keys do.
  A left click selects a row: `ui::list` records where the list was drawn and its scroll offset (`ui::ListArea`), and
  `tui.rs` maps the click to `Click::Row`. Mouse capture is released around `$EDITOR` and in the panic hook.
- **Mouse on the desktop:** the wheel scrolls normally; the selection is scrolled into view only after the keyboard or
  a button moved it. Double-clicking a row is `Click::OpenRow`, and clicking a `?` menu entry is `Click::Binding`.
- **Mouse in text (desktop):** `ui::pointer` keeps each drawn line's GPUI `TextLayout` (`Hit`) to map the mouse to a
  character, and calls `App::point` with an `editor::Pointer` (place, extend, word, line). Drags use window-wide
  listeners, so they keep selecting outside the field; `TaskerView::drag` names the field. The core editors remember
  a double- or triple-clicked word or line (`Grab`), so dragging on extends by whole words or lines. On Linux,
  `TaskerView::share_selection` copies the selection to the primary selection after each input.
- **Right-click menus (desktop):** `ui::menu`. A right-click first selects its target (`App::click`); entries run a key
  through `TaskerView::press` or a `ClipboardOp`, so a menu can't do anything a button or key can't.

### Adding a key

1. Handle it in the right `key_*` function in `app/input.rs` (the action itself goes in `app/actions.rs`).
2. Add a `Binding` to `LIST`, `ISSUE` or `TAGS` in `keys.rs`: the footer hint and the `?` menu entry come from there.
   The footer hint is a `(key, label)` pair. `keys::mnemonic` marks the key inside the label (`(n)ew`), preferring the
   start of a word, and falls back to `(key) label`. Pick labels that contain the key when you can.
3. In the desktop app, add a button where the action belongs: `ui::action(id, label, kind, code, cx)` makes a button that
   presses the key, with the key in its tooltip (`action_after` when it acts on a comment or tag row: it clicks the row first).
4. Document it in [`keybindings.md`](keybindings.md) and the relevant page.

### Changing the file format

`model/markdown.rs` documents the format at the top and holds both the writer and the parser; change them together and
extend the `round_trip` test. The parser ignores unknown `- key:` lines and unknown `##` sections.
Until the first release, the format may change freely (migrate existing files with the app's own writer).
After a release, keep it backward compatible.

## Tests

Tests live in each crate's `tests/` folder, one file per module, laid out like `src/`: `core/src/editor.rs` is tested
by `core/tests/editor.rs`, `core/src/model/mod.rs` by `core/tests/model.rs`. They are unit tests with access to private
items: the module includes its test file with a hook at its end, and the file starts with `use super::*`:

```rust
#[cfg(test)]
#[path = "../tests/editor.rs"]
mod tests;
```

The path is relative to the module's folder. `autotests = false` in each crate stops Cargo from also building these
files as integration tests. Keep them inside the crate: rust-analyzer doesn't follow `#[path]` out of it.
`core/tests/app.rs` drives the whole app with key presses, button presses and clicks against a temporary data folder.

| module | covers |
| --- | --- |
| `app` | status keys and history, new tasks as todo, delete confirmation, comments add/edit/delete, `ctrl+c`, clipboard (select all, copy, cut, paste), unlisted keys do nothing and listed keys do something, search, tag rename/merge/delete/recolor, buttons leaving the search box, clicks selecting, opening, filtering, focusing fields and running `?` entries, row buttons acting on their own row, mouse selection |
| `model` | file round trip, hand-typed history, missing id, status changes recorded once, search matching, tags, slugs, heading rules, timestamps, ages |
| `store` | renaming on title change, unreadable files become warnings, `tags.md` isn't a task |
| `tags` | no color repeats until the palette is used up, `tags.md` round trip, custom colors, rename/remove/cycle |
| `editor` | line editing (split/join), unicode, wrapping and scrolling; one-line cursor editing and horizontal scroll; selecting, copying and replacing a selection in both; mouse selection by character, word and line |
| `keys` | mnemonic hint splitting |
| `cli` | commands, list options and output |
| `enums` | generated `ALL` / `position` / `next`, `parse` as the inverse of `as_str` |
| `ui` (tui) | wrapping, truncation, popup placement |
| `ui` (gui) | GPUI keystrokes to core keys, shortcut names for tooltips, typed keys, clipboard shortcuts, drag selection, right-click menus, middle-click paste (Linux) and the cursor's blink in a real (test) window |

To check the UI by hand, run it in `tmux` against a scratch folder and capture the screen:

```sh
tmux new-session -d -s t -x 110 -y 30 "TASKER_DIR=/tmp/t ./target/debug/tasker"   # as root: prefix TASKER_ALLOW_ROOT=1
tmux send-keys -t t n; tmux send-keys -t t -l "demo"; tmux send-keys -t t Enter
tmux capture-pane -pt t
```

The desktop app is checked by running it (`TASKER_DIR=/tmp/t cargo run -p tasker-gui`) and looking at it.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `main` and every pull request:

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` on Linux, macOS and Windows
- `cargo test` (core and terminal UI) with the minimum supported Rust version (1.88)

## Releases and installers

### Publishing a release

1. Update `version` under `[workspace.package]` in `Cargo.toml` (and run `cargo build` so `Cargo.lock` follows) and add the
   version to `CHANGELOG.md`. All three crates share it.
2. Commit, then create a signed, annotated tag and push it:

   ```sh
   git tag -s vX.Y.Z -m "vX.Y.Z: <summary>"
   git push origin main vX.Y.Z
   ```

`.github/workflows/release.yml` then checks that the tag matches `Cargo.toml`, builds both apps for every platform
(Linux x86_64 and ARM64, macOS universal, Windows x64), and creates the GitHub release with the
assets below, their `.sha256` files, and `install.sh` / `install.ps1`. From then on the one-line installers work.
Each app and platform is a separate job (`tasker · linux x86_64`, `tasker-gui · macos universal`, …), so one failing build
doesn't stop the others, and their files can still be downloaded from the run. The release is only published when all
of them succeed, so the installers never find a file missing.

The installers (`install.sh`, `install.ps1`) download files from the project's **releases**. A release must contain
these assets, each with a matching `<file>.sha256` (the output of `sha256sum <file>`):

| asset | contents | build target |
| --- | --- | --- |
| `tasker-x86_64-unknown-linux-musl.tar.gz` | `tasker` | `x86_64-unknown-linux-musl` |
| `tasker-aarch64-unknown-linux-musl.tar.gz` | `tasker` | `aarch64-unknown-linux-musl` |
| `tasker-universal-apple-darwin.tar.gz` | `tasker` (universal: `lipo` of x86_64 + aarch64) | `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| `tasker-x86_64-pc-windows-msvc.zip` | `tasker.exe` | `x86_64-pc-windows-msvc` |
| `tasker-gui-x86_64-unknown-linux-gnu.tar.gz` | `tasker-gui` | `x86_64-unknown-linux-gnu`, built on Ubuntu 22.04 (glibc 2.35) |
| `tasker-gui-aarch64-unknown-linux-gnu.tar.gz` | `tasker-gui` | `aarch64-unknown-linux-gnu`, built on Ubuntu 22.04 |
| `tasker-gui-universal-apple-darwin.tar.gz` | `tasker-gui` (universal) | `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| `tasker-gui-x86_64-pc-windows-msvc.zip` | `tasker-gui.exe` | `x86_64-pc-windows-msvc` |

The terminal UI is static on Linux (musl), so it runs anywhere. The desktop app can't be: it loads the system's graphics
libraries (Vulkan, Wayland/X11, xkbcommon, fontconfig), so it's built against glibc on the oldest supported Ubuntu.

Archives contain the binary at the root, not in a subfolder. macOS builds need a macOS machine or runner (Apple SDK).
Linux and Windows can be cross-compiled from Linux, e.g. for a quick local check:

```sh
rustup target add x86_64-unknown-linux-musl x86_64-pc-windows-gnu   # needs musl-tools, mingw-w64
cargo build --release -p tasker --target x86_64-unknown-linux-musl
cargo build --release -p tasker --target x86_64-pc-windows-gnu
```

### Pointing the installers at your repository

They are set to this repository (`singudotdev/tasker` on GitHub). For a fork, edit the two lines near the top of **both** scripts:

```sh
REPO="${TASKER_REPO:-singudotdev/tasker}"     # install.sh
HOST="${TASKER_HOST:-github}"           # github | gitlab
```

```powershell
$repo = ... 'singudotdev/tasker'              # install.ps1
$hostKind = ... 'github'
```

| host | "latest" download URL used |
| --- | --- |
| GitHub | `https://github.com/<repo>/releases/latest/download/<asset>` |
| GitLab | `https://gitlab.com/<repo>/-/releases/permalink/latest/downloads/<asset>` (asset links must use `filepath: /<asset>`) |

Self-hosted GitLab: set `TASKER_GITLAB_URL`. Any other server: set `TASKER_BASE_URL` to the folder containing the assets.
The same variables are handy for testing against a local folder:

```sh
cd dist && python3 -m http.server 8000 &
TASKER_BASE_URL=http://127.0.0.1:8000 sh install.sh --app both --dir ~/tk-test
```

Lint the shell installer with `shellcheck install.sh` (it's kept clean).

The purge logic in both scripts must stay in sync with what the app writes: task files are named `NNNN-slug.md`
(`store::Store::save`), plus `tags.md` and `.NNNN-slug.md.tmp` temp files. If you add a new kind of file to the data folder,
add it to `owned_files` in `install.sh` and to the matching filter in `install.ps1`.

## Contributing

1. Open an issue describing the problem or idea.
2. Keep pull requests focused. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings) and
   `cargo test --workspace`.
3. Update the docs in `docs/` when you change behavior or keys. A new action usually needs a key (both apps) and a button
   (desktop app); see [Adding a key](#adding-a-key).
4. Don't break existing data folders. See [Changing the file format](#changing-the-file-format).
