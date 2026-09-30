#!/bin/sh
# tasker installer for Linux and macOS.
#
#   curl -fsSL <url>/install.sh | sh
#   wget -qO-  <url>/install.sh | sh
#
# Options (after `sh -s --` when piping):
#   --app tui|gui|both which app: the terminal UI `tasker` (default), the desktop app `tasker-gui`, or both.
#                      Both use the same tasks. With --uninstall/--purge the default is both.
#   --version vX.Y.Z   install a specific release (default: latest)
#   --dir PATH         install directory inside your home (default: ~/.local/bin)
#   --uninstall        remove the app(s) (your tasks are kept)
#   --purge            remove the app(s) AND your data: tasks and tags.md
#                      (asks for confirmation; add --yes to skip it)
#
# tasker is installed and used per user: this script refuses to run as root/sudo and only
# installs inside $HOME. (If root is your only user, e.g. in a container: TASKER_ALLOW_ROOT=1.)
#
# Environment overrides: TASKER_APP, TASKER_VERSION, TASKER_INSTALL_DIR, TASKER_REPO,
# TASKER_HOST (github | gitlab), TASKER_GITLAB_URL, TASKER_BASE_URL, TASKER_ALLOW_ROOT.

set -eu

# ---- where releases are published: edit these two lines for your repository ----
REPO="${TASKER_REPO:-singudotdev/tasker}"
HOST="${TASKER_HOST:-github}"

APP="${TASKER_APP:-}"
VERSION="${TASKER_VERSION:-latest}"
INSTALL_DIR="${TASKER_INSTALL_DIR:-$HOME/.local/bin}"
UNINSTALL=0
PURGE=0
YES=0

say() { printf '%s\n' "tasker: $*"; }
die() { printf '%s\n' "tasker: error: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --app) [ $# -ge 2 ] || die "--app needs a value"; APP="$2"; shift 2 ;;
        --version) [ $# -ge 2 ] || die "--version needs a value"; VERSION="$2"; shift 2 ;;
        --dir) [ $# -ge 2 ] || die "--dir needs a value"; INSTALL_DIR="$2"; shift 2 ;;
        --uninstall) UNINSTALL=1; shift ;;
        --purge) UNINSTALL=1; PURGE=1; shift ;;
        -y | --yes) YES=1; shift ;;
        -h | --help)
            printf '%s\n' "usage: install.sh [--app tui|gui|both] [--version vX.Y.Z] [--dir PATH] [--uninstall | --purge [--yes]]"
            exit 0
            ;;
        *) die "unknown option: $1" ;;
    esac
done

# ---- per-user only ----
if [ "$(id -u)" = 0 ] && [ "${TASKER_ALLOW_ROOT:-}" != 1 ]; then
    die "tasker is installed per user: run this as your normal user, not as root or with sudo.
       (Only if root is your only user, e.g. a container, set TASKER_ALLOW_ROOT=1.)"
fi
case "$INSTALL_DIR" in /*) ;; *) INSTALL_DIR="$PWD/$INSTALL_DIR" ;; esac
INSTALL_DIR=${INSTALL_DIR%/}
case "$INSTALL_DIR" in
    *..*) die "install folder must be a plain path without '..': $INSTALL_DIR" ;;
esac
case "$INSTALL_DIR/" in
    "${HOME%/}"/*) ;;
    *) die "install folder must be inside your home folder (${HOME%/}), got $INSTALL_DIR" ;;
esac

# ---- which apps ----
if [ -z "$APP" ]; then
    if [ "$UNINSTALL" = 1 ]; then APP=both; else APP=tui; fi
fi
case "$APP" in
    tui) BINS="tasker" ;;
    gui) BINS="tasker-gui" ;;
    both) BINS="tasker tasker-gui" ;;
    *) die "--app must be tui, gui or both, got $APP" ;;
esac

# The desktop app's entry in the Linux application menu.
DESKTOP_FILE="${XDG_DATA_HOME:-$HOME/.local/share}/applications/tasker-gui.desktop"

# Where tasker keeps its data: ask an installed binary, else mirror its defaults.
data_dir() {
    for bin in "$INSTALL_DIR/tasker" "$INSTALL_DIR/tasker-gui"; do
        if [ -x "$bin" ] && dir=$("$bin" path 2>/dev/null) && [ -n "$dir" ]; then
            printf '%s\n' "$dir"
            return
        fi
    done
    if [ -n "${TASKER_DIR:-}" ]; then
        printf '%s\n' "$TASKER_DIR"
    elif [ "$(uname -s)" = Darwin ]; then
        printf '%s\n' "$HOME/Library/Application Support/tasker"
    else
        printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/tasker"
    fi
}

# Files tasker creates: NNNN-slug.md tasks, tags.md, and leftover temp files.
owned_files() {
    find "$1" -maxdepth 1 -type f \( -name '[0-9][0-9][0-9][0-9]*-*.md' -o -name 'tags.md' -o -name '.*.md.tmp' \)
}

confirm() {
    [ "$YES" = 1 ] && return 0
    # stdin is the script itself when piped from curl/wget, so ask on the terminal.
    if { true </dev/tty; } 2>/dev/null; then
        printf 'tasker: %s [y/N] ' "$1" >/dev/tty
        read -r answer </dev/tty || answer=""
        case "$answer" in y | Y | yes | YES) return 0 ;; esac
        return 1
    fi
    die "no terminal to confirm on; re-run with --yes to delete your data"
}

if [ "$UNINSTALL" = 1 ]; then
    if [ "$PURGE" = 1 ]; then
        data=$(data_dir)
        case "$data" in
            "" | / | "$HOME" | "$HOME/") die "refusing to delete data in '$data'" ;;
        esac
        if [ -d "$data" ]; then
            tasks=$(owned_files "$data" | grep -vc -e '/tags\.md$' -e '\.tmp$' || true)
            say "data folder: $data"
            say "this permanently deletes $tasks task file(s) and tags.md"
            confirm "delete your tasker data?" || die "cancelled, nothing was removed"
        else
            say "no data folder at $data"
            data=""
        fi
    fi

    for bin in $BINS; do
        if [ -e "$INSTALL_DIR/$bin" ]; then
            rm -f "$INSTALL_DIR/$bin"
            say "removed $INSTALL_DIR/$bin"
        else
            say "no binary at $INSTALL_DIR/$bin"
        fi
        if [ "$bin" = tasker-gui ] && [ -e "$DESKTOP_FILE" ]; then
            rm -f "$DESKTOP_FILE"
            say "removed $DESKTOP_FILE"
        fi
    done

    if [ "$PURGE" = 1 ] && [ -n "$data" ]; then
        owned_files "$data" | while IFS= read -r f; do rm -f "$f"; done
        if rmdir "$data" 2>/dev/null; then
            say "removed $data"
        else
            say "deleted tasker's files; kept $data because it contains other files"
        fi
    elif [ "$PURGE" = 0 ]; then
        say "your tasks were kept in $(data_dir) (use --purge to delete them too)"
    fi
    exit 0
fi

# ---- detect platform ----
os=$(uname -s)
arch=$(uname -m)
case "$arch" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *) die "unsupported CPU architecture: $arch" ;;
esac
case "$os" in
    Linux | Darwin) ;;
    MINGW* | MSYS* | CYGWIN*) die "on Windows, use the PowerShell installer: irm <url>/install.ps1 | iex" ;;
    *) die "unsupported operating system: $os" ;;
esac

# The release file for one app. On Linux the terminal UI is static (musl); the desktop app links the
# system's graphics libraries (glibc).
asset_for() {
    case "$os:$1" in
        Linux:tasker) printf '%s\n' "tasker-$arch-unknown-linux-musl.tar.gz" ;;
        Linux:tasker-gui) printf '%s\n' "tasker-gui-$arch-unknown-linux-gnu.tar.gz" ;;
        Darwin:*) printf '%s\n' "$1-universal-apple-darwin.tar.gz" ;;
    esac
}

# ---- download location ----
if [ -n "${TASKER_BASE_URL:-}" ]; then
    base="$TASKER_BASE_URL"
else
    case "$HOST" in
        github)
            if [ "$VERSION" = latest ]; then
                base="https://github.com/$REPO/releases/latest/download"
            else
                base="https://github.com/$REPO/releases/download/$VERSION"
            fi
            ;;
        gitlab)
            gl="${TASKER_GITLAB_URL:-https://gitlab.com}"
            if [ "$VERSION" = latest ]; then
                base="$gl/$REPO/-/releases/permalink/latest/downloads"
            else
                base="$gl/$REPO/-/releases/$VERSION/downloads"
            fi
            ;;
        *) die "TASKER_HOST must be github or gitlab" ;;
    esac
fi

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q -O "$2" "$1"; }
else
    die "curl or wget is required"
fi

tmp=$(mktemp -d 2>/dev/null || mktemp -d -t tasker)
trap 'rm -rf "$tmp"' EXIT INT TERM

mkdir -p "$INSTALL_DIR" || die "cannot create $INSTALL_DIR (try --dir)"
[ -w "$INSTALL_DIR" ] || die "$INSTALL_DIR is not writable (choose another folder in your home with --dir)"

# Downloads, verifies and installs one app.
install_app() {
    bin=$1
    asset=$(asset_for "$bin")
    say "downloading $asset ($VERSION)"
    fetch "$base/$asset" "$tmp/$asset" || die "download failed: $base/$asset"

    # ---- verify checksum ----
    if fetch "$base/$asset.sha256" "$tmp/$asset.sha256" 2>/dev/null; then
        expected=$(cut -d' ' -f1 <"$tmp/$asset.sha256")
        if command -v sha256sum >/dev/null 2>&1; then
            actual=$(sha256sum "$tmp/$asset" | cut -d' ' -f1)
        elif command -v shasum >/dev/null 2>&1; then
            actual=$(shasum -a 256 "$tmp/$asset" | cut -d' ' -f1)
        else
            actual=""
            say "warning: no sha256sum/shasum found, skipping checksum verification"
        fi
        if [ -n "$actual" ]; then
            [ "$actual" = "$expected" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"
            say "checksum ok"
        fi
    else
        say "warning: no checksum published for $asset, skipping verification"
    fi

    # ---- install ----
    tar -xzf "$tmp/$asset" -C "$tmp" || die "could not extract $asset"
    [ -f "$tmp/$bin" ] || die "archive does not contain a '$bin' binary"
    # Replace atomically so a running copy isn't disturbed.
    cp "$tmp/$bin" "$INSTALL_DIR/$bin.new"
    chmod 755 "$INSTALL_DIR/$bin.new"
    mv -f "$INSTALL_DIR/$bin.new" "$INSTALL_DIR/$bin"
    if [ "$os" = Darwin ]; then
        xattr -d com.apple.quarantine "$INSTALL_DIR/$bin" 2>/dev/null || true
    fi
    say "installed $("$INSTALL_DIR/$bin" --version 2>/dev/null || echo "$bin") to $INSTALL_DIR/$bin"

    # The desktop app goes in the Linux application menu.
    if [ "$bin" = tasker-gui ] && [ "$os" = Linux ]; then
        mkdir -p "$(dirname "$DESKTOP_FILE")"
        cat >"$DESKTOP_FILE" <<EOF_DESKTOP
[Desktop Entry]
Type=Application
Name=tasker
GenericName=Todo tracker
Comment=Tasks in todo, doing, parked and done, stored as markdown files
Exec="$INSTALL_DIR/tasker-gui"
Terminal=false
Categories=Office;ProjectManagement;
EOF_DESKTOP
        say "added tasker to your application menu ($DESKTOP_FILE)"
    fi
}

for bin in $BINS; do
    install_app "$bin"
done

# ---- PATH hint ----
case ":$PATH:" in
    *":$INSTALL_DIR:"*) say "run: $(printf '%s\n' "$BINS" | sed 's/ / or /')" ;;
    *)
        say "$INSTALL_DIR is not in your PATH. Add it with:"
        # shellcheck disable=SC2016 # $PATH is meant literally in the printed hint
        case "${SHELL:-sh}" in
            */fish | fish) printf '    fish_add_path %s\n' "$INSTALL_DIR" ;;
            */zsh | zsh) printf '    echo '\''export PATH="%s:$PATH"'\'' >> ~/.zshrc && exec zsh\n' "$INSTALL_DIR" ;;
            *) printf '    echo '\''export PATH="%s:$PATH"'\'' >> ~/.bashrc && exec bash\n' "$INSTALL_DIR" ;;
        esac
        ;;
esac
