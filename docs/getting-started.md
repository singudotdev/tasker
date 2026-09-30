# Getting started

## Install with the installer (recommended)

tasker comes as two apps that share your tasks. Install either one, or both:

| app | command | what it is |
| --- | --- | --- |
| **terminal UI** (default) | `tasker` | keyboard-first, lazygit style; works over SSH. One self-contained file, about 1 MB, no dependencies |
| **desktop app** | `tasker-gui` | a window with buttons, dialogs and double-click to open; the terminal's keys work there too |

The installer picks the right file for your system, verifies its SHA-256 checksum and installs it for **your user only**.
The desktop app also gets an entry in your application menu (Linux) or Start Menu (Windows).

> **tasker is a per-user app.** Each user installs their own copy in their home folder and has their own tasks.
> - The installers **refuse to run as root / with `sudo`** (Linux, macOS) or **from an elevated "Run as administrator" PowerShell** (Windows).
> - The install folder must be **inside your home folder** (Linux/macOS) or **user profile** (Windows). System folders such as `/usr/local/bin` or `Program Files` are rejected.
> - The app itself **refuses to run as root**, so `sudo tasker` can't create root-owned files in your data folder.
>
> If root is genuinely your only user, e.g. in a container, opt in with `TASKER_ALLOW_ROOT=1`
> (for both the installer and the app). The Windows equivalent for admin-only accounts is `$env:TASKER_ALLOW_ADMIN = "1"`.

### Linux and macOS

```sh
curl -fsSL https://raw.githubusercontent.com/singudotdev/tasker/main/install.sh | sh
```

No curl? Use `wget -qO- … | sh`. Only a POSIX `sh` is required (bash, dash, busybox and zsh all work).

Options go after `sh -s --`:

```sh
curl -fsSL …/install.sh | sh -s -- --app gui            # the desktop app instead of the terminal UI
curl -fsSL …/install.sh | sh -s -- --app both           # both
curl -fsSL …/install.sh | sh -s -- --version v0.1.0     # a specific release
curl -fsSL …/install.sh | sh -s -- --dir ~/bin          # another folder in your home (default ~/.local/bin)
```

Uninstalling uses the same script, see [Uninstalling](#uninstalling).

If the install folder isn't on your `PATH`, the installer prints the exact line to add for your shell.

### Windows

In PowerShell (Windows PowerShell 5.1 or PowerShell 7):

```powershell
irm https://raw.githubusercontent.com/singudotdev/tasker/main/install.ps1 | iex
```

It installs to `%LOCALAPPDATA%\Programs\tasker\tasker.exe` (and `tasker-gui.exe` for the desktop app) and adds that folder
to your **user** `PATH`, so open a new terminal afterwards.
Options are environment variables:

```powershell
$env:TASKER_APP = "gui"; irm …/install.ps1 | iex                # the desktop app ("both" for both)
$env:TASKER_VERSION = "v0.1.0"; irm …/install.ps1 | iex        # a specific release
$env:TASKER_INSTALL_DIR = "$HOME\tools"; irm …/install.ps1 | iex  # another folder in your profile
```

Windows on ARM uses the x64 build through Windows' built-in emulation.

### Supported platforms

| OS | CPU | terminal UI | desktop app |
| --- | --- | --- | --- |
| Linux | x86_64, ARM64 | `tasker-<cpu>-unknown-linux-musl.tar.gz` (any distro, statically linked) | `tasker-gui-<cpu>-unknown-linux-gnu.tar.gz` (glibc 2.35+, Wayland or X11) |
| macOS 11+ | Intel and Apple Silicon (universal) | `tasker-universal-apple-darwin.tar.gz` | `tasker-gui-universal-apple-darwin.tar.gz` |
| Windows 10/11 | x64 (ARM64 via emulation) | `tasker-x86_64-pc-windows-msvc.zip` | `tasker-gui-x86_64-pc-windows-msvc.zip` |

`<cpu>` is `x86_64` or `aarch64`. You can also download these files by hand from the releases page, unpack them,
and put `tasker` / `tasker-gui` anywhere on your `PATH`.

The desktop app on Linux uses your desktop's graphics libraries: Vulkan (`libvulkan1` / `vulkan-icd-loader`), `libxkbcommon`,
and Wayland or X11. Desktop installs normally have them already. On Windows it opens no console window, so use
`tasker` there for the command line (`list`, `path`).

### Updating

Run the installer again, with the same `--app`. It replaces the binaries; your data isn't touched.

## Uninstalling

There are two levels:

| | removes the program | removes your tasks and `tags.md` |
| --- | :---: | :---: |
| **uninstall** | ✓ | |
| **purge** (full uninstall) | ✓ | ✓ |

**Linux / macOS**

Both levels remove both apps; add `--app tui` or `--app gui` (`$env:TASKER_APP` on Windows) to remove just one.

```sh
curl -fsSL …/install.sh | sh -s -- --uninstall     # program only
curl -fsSL …/install.sh | sh -s -- --purge         # program + data (asks first)
curl -fsSL …/install.sh | sh -s -- --purge --yes   # no question, for scripts
```

**Windows**

```powershell
$env:TASKER_UNINSTALL = "1"; irm …/install.ps1 | iex                  # program only (also removes it from PATH)
$env:TASKER_PURGE = "1"; irm …/install.ps1 | iex                      # program + data (asks first)
$env:TASKER_PURGE = "1"; $env:TASKER_YES = "1"; irm …/install.ps1 | iex   # no question
```

What purge does:

- **It finds your data** the same way tasker does, by asking an installed binary (`tasker path`). So `TASKER_DIR` is respected.
- **It shows what will be deleted and asks `[y/N]`.** Anything but `y` cancels, and nothing is removed.
  Without a terminal to ask on (e.g. in CI), it refuses unless you pass `--yes` / `TASKER_YES`.
- **It deletes only what tasker created:** task files (`NNNN-*.md`), `tags.md` and leftover temp files.
  If the folder has anything else in it (for example your `TASKER_DIR` is inside a notes vault or a git repository), those files and the folder are **kept**.
- **It refuses** to work on your home folder or a drive root.

> Purge can't be undone. To keep a copy first: `tar -czf tasker-backup.tar.gz -C "$(tasker path)" .`
> (Windows: `Compress-Archive "$(tasker path)\*" tasker-backup.zip`)

If you installed with `cargo install`, remove the programs with `cargo uninstall tasker` / `cargo uninstall tasker-gui`.
Your data then stays in the [data folder](#where-your-data-lives); delete it by hand if you want.

## Build from source

### Requirements

- **Rust 1.88 or newer** for the terminal UI; the desktop app needs a recent stable Rust. Install it with [rustup](https://rustup.rs):
  - Linux / macOS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Arch Linux: `sudo pacman -S rustup && rustup default stable`
  - Windows: download `rustup-init.exe` from [rustup.rs](https://rustup.rs)
- A terminal with Unicode support. 256 colors is enough; truecolor is only needed for [custom hex tag colors](tasks-and-tags.md#tag-colors).
- For the desktop app on Linux, the development packages of its graphics libraries, e.g. on Debian/Ubuntu
  `sudo apt install libfontconfig-dev libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev libvulkan1`
  (other distributions: see the [Zed Linux build docs](https://zed.dev/docs/development/linux), which use the same libraries).
  On macOS it needs Xcode (for the Metal shader compiler).

### Build and install

From a clone of the repository:

```sh
cargo install --path tui     # the terminal UI: tasker
cargo install --path gui     # the desktop app: tasker-gui
```

This builds an optimized binary and puts it in `~/.cargo/bin/` (`%USERPROFILE%\.cargo\bin\` on Windows).
Make sure that directory is on your `PATH`:

```sh
# bash / zsh
export PATH="$HOME/.cargo/bin:$PATH"
# fish
fish_add_path ~/.cargo/bin
```

To build without installing:

```sh
cargo build --release                  # → target/release/tasker
cargo build --release -p tasker-gui    # → target/release/tasker-gui
```

## First run

```sh
tasker
```

1. Press `n` to create a task. Type a title, press `tab` to add tags (optional, e.g. `feature, backend`), then press `enter`.
   The task starts as **todo**.
2. Press `s` when you start working on it: it moves to **doing**, at the top of the list.
3. Press `m` to write a description, or `c` to add a comment as you go.
4. Press `d` when it's finished. Done tasks are hidden; `f` shows them again.
5. Press `v` to see the whole task: description, comments, and the history of when its status changed.
6. Press `q` to quit. Everything is already saved.

Press `?` at any time to see all the keys.

In the desktop app (`tasker-gui`) it's all buttons: **+ New task** in the toolbar, then the status buttons
(To do, Doing, Parked, Done) and the Edit / Add buttons in the details panel. Double-click a task to open it,
or right-click it for a menu of its actions.
The same keys work too, and every button's tooltip names its key.

## Where your data lives

| platform | default folder |
| --- | --- |
| Linux | `~/.local/share/tasker` (or `$XDG_DATA_HOME/tasker`) |
| macOS | `~/Library/Application Support/tasker` |
| Windows | `%APPDATA%\tasker` (e.g. `C:\Users\you\AppData\Roaming\tasker`) |

`tasker path` prints the folder in use.

To keep your data somewhere else (a notes vault, a git repository, a synced folder), set `TASKER_DIR`:

```sh
export TASKER_DIR=~/notes/todo            # bash / zsh
set -Ux TASKER_DIR ~/notes/todo           # fish
$env:TASKER_DIR = "D:\notes\todo"         # PowerShell (current session)
```

The folder is created if it doesn't exist. See [Data format](data-format.md) for what goes inside.

## Trying it without touching your real data

```sh
TASKER_DIR=/tmp/tasker-test tasker
```

## Next steps

- [Tasks, tags & search](tasks-and-tags.md): statuses, history, tags and comments
- [Keybindings](keybindings.md)
