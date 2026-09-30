# Contributing

Thanks for helping out! Everything you need to build, test and understand the code is in
[docs/development.md](docs/development.md).

In short:

```sh
cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace   # clippy must be warning-free
TASKER_DIR=/tmp/tasker-dev cargo run                  # try your change in the terminal UI without touching real data
TASKER_DIR=/tmp/tasker-dev cargo run -p tasker-gui    # … and in the desktop app
```

The shared logic is in `core/`, the terminal UI in `tui/` and the desktop app in `gui/`. Plain `cargo test` skips the
desktop app (it needs GPUI and, on Linux, some graphics libraries: see [docs/development.md](docs/development.md)).

- Keep pull requests focused, and describe the behavior change in the description.
- Update `docs/` when keys or behavior change.
- File format changes: see [Changing the file format](docs/development.md#changing-the-file-format). After the first release, existing data folders must keep loading.
