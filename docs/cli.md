# Command line

```
tasker           open the TUI
tasker list [doing|todo|parked|done|all] [--tag TAG]
                 print tasks as markdown, one section per status
tasker path      print the data directory
tasker --version print the version (also -V)
tasker --help    show usage
```

All commands read and write the same folder as the TUI (see [where your data lives](getting-started.md#where-your-data-lives)),
so you can run them while the TUI is open.

## `tasker list`

Prints tasks as a markdown list, one `## status` section per status, most recently changed first. Sections without tasks are left out.

```
$ tasker list
## doing

- #2 Fix login 500 #prod #support

## todo

- #4 Review Ana's PR #support
- #1 Feature: export CSV #feature #backend
```

| option | meaning | default |
| --- | --- | --- |
| `doing` / `todo` / `parked` / `done` | only these statuses; give several to combine them | all but `done` |
| `all` | every status, done included | |
| `--tag TAG` | only tasks with this tag (`prod` or `#prod`) | all |

Examples:

```sh
tasker list                     # everything not done yet
tasker list done --tag client-a # what's finished for one client
tasker list all | wl-copy       # copy to the clipboard (Wayland); xclip, pbcopy or clip elsewhere
tasker list doing | grep -c '^- '   # how many tasks are in progress
```

## `tasker path`

Prints the data folder, e.g. `/home/you/.local/share/tasker`. Handy for scripts: `cd "$(tasker path)"`.

## Environment variables

| variable | purpose |
| --- | --- |
| `TASKER_DIR` | data folder, overriding the platform default |
| `TASKER_ALLOW_ROOT` | set to `1` to allow running as root. tasker is per-user and refuses root by default; only use this where root is the only user, e.g. containers |
| `VISUAL`, `EDITOR` | editor for `o`, checked in that order. It may include arguments, e.g. `code -w`. The fallback is `vi`, or `notepad` on Windows |

## Exit codes

`0` on success. Non-zero if an error occurred (for example an unreadable data folder or an unknown option), with a message on stderr.
