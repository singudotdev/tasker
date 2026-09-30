# Integrations

## Status bars

`tasker list doing` reads the files directly, so it's fast and works whether or not the TUI is open.
Counting its lines gives you the number of tasks in progress:

```sh
tasker list doing | grep -c '^- '
```

### waybar

```jsonc
// ~/.config/waybar/config
"modules-right": ["custom/tasker", "clock"],
"custom/tasker": {
    "exec": "tasker list doing | grep -c '^- '",
    "interval": 30,
    "format": "▶ {} doing",
    "on-click": "foot -e tasker"      // your terminal
}
```

### tmux

```tmux
# ~/.tmux.conf
set -g status-right "#(tasker list doing | grep -c '^- ') doing | %H:%M"
```

### Show the first task you're doing

```sh
tasker list doing | sed -n 's/^- //p' | head -1
```

Use it the same way in polybar, i3blocks, starship or SwiftBar.

## Syncing between machines

Your data is a folder of small text files, so you can sync it any way you sync files. Point `TASKER_DIR` at the synced folder on each machine.

### git

```sh
export TASKER_DIR=~/todo
cd ~/todo && git init
# whenever you like
git add -A && git commit -m "todo $(date +%F)" && git push
```

Each task is its own file and writes are small, so merges rarely conflict.

### Syncthing, Dropbox, iCloud, OneDrive, Nextcloud

These work as is. If the sync tool creates conflict copies (`… (conflicted copy).md`), tasker loads them as tasks too.
Delete the one you don't want.

### Obsidian and other notes apps

Set `TASKER_DIR` to a folder inside your vault, e.g. `~/vault/todo`. Task files render as regular notes,
and you can link to them. Their `## Description`, `## Comments` and `## History` sections read like normal notes.
