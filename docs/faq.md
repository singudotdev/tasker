# FAQ & troubleshooting

### Can several tasks be in "doing" at the same time?

Yes. `doing` just means you've started it. Press `p` to park a task you've set aside, or `b` to send it back to `todo`.

### Where are my tasks?

Run `tasker path`. The default is `~/.local/share/tasker` on Linux, `~/Library/Application Support/tasker` on macOS,
and `%APPDATA%\tasker` on Windows. `TASKER_DIR` overrides it.

### A footer message says "skipped N file(s)"

Some `.md` files in the data folder couldn't be read as tasks. The message names each file and the reason (usually a missing `- id:` line).
tasker doesn't modify those files. Fix them or move them out of the folder, then press `R`.

### Tag colors look wrong or are missing

- Palette colors need a 256-color terminal. Practically every modern terminal is one. Check that `TERM` isn't `dumb`, `vt100` or similar.
- Hex colors (`#33aaff`) need **truecolor**. If your terminal doesn't support it, use a palette name instead.
- If you edited `tags.md`, press `R` to reload.

### I can't select text with the mouse

tasker uses the mouse (wheel, click), like lazygit, so the terminal passes mouse events to it.
Hold `shift` while dragging to select text; on macOS use `fn` or `option`, depending on the terminal.

In the desktop app, dragging doesn't select text yet: in a field or the editor, select with `shift` and the arrows or
`ctrl+a`, then copy with `ctrl+c` (see [Keybindings](keybindings.md)).

### Icons (`▶ · ‖ ✓`) show as boxes

Your terminal font lacks those characters. Any font with good Unicode coverage fixes it, e.g. a Nerd Font, JetBrains Mono, DejaVu Sans Mono or Cascadia Code.

### `o` doesn't open my editor

tasker uses `$VISUAL`, then `$EDITOR`, then `vi` (`notepad` on Windows). GUI editors must **wait** until the file is closed:

```sh
export EDITOR="code -w"        # VS Code
export EDITOR="subl -w"        # Sublime Text
export EDITOR="zed --wait"     # Zed
```

### Can I fix the date of a status change?

Yes. Open the task with `o` and change the time in its `## History` line. See [Data format](data-format.md#editing-by-hand).

### How do I rename a tag everywhere?

Press `t` for the tags screen, select the tag and press `r`. Renaming it to a tag that already exists merges the two.
See [Managing tags](tasks-and-tags.md#managing-tags).

### Does it work on Windows?

Yes. It works in Windows Terminal, PowerShell and cmd. Data goes to `%APPDATA%\tasker` and `o` opens Notepad unless `EDITOR` is set.

### Why does the list say `2d` when it's been almost 3 days?

The **since** column rounds down: minutes, then hours, days and weeks. The exact time is in the details panel and the history.

### "tasker is a per-user app and doesn't run as root"

tasker is installed and used per user, so run it as your normal account, without `sudo`. Running it as root would either
use root's data folder instead of yours, or leave root-owned files in yours.
If root really is your only user (e.g. a Docker/Podman container), set `TASKER_ALLOW_ROOT=1`.
The same applies to the installer. On Windows, run the installer from a normal (non-administrator) PowerShell.

### Can I install it for all users on a machine?

No, on purpose. Each user runs the installer themselves and gets their own copy and their own tasks.
It's about 1 MB, so the duplication doesn't matter.

### How do I uninstall, or remove all my data?

`--uninstall` removes only the program. `--purge` also deletes your tasks, after asking. It never deletes files that tasker didn't create.
See [Uninstalling](getting-started.md#uninstalling).

### How do I back up my data?

Copy the data folder, or [keep it in git](integrations.md#git). It's all plain text.
