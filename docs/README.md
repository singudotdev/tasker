# tasker documentation

tasker is a todo tracker with two apps over the same tasks: a terminal UI (`tasker`) and a desktop app (`tasker-gui`).
Tasks move between **todo**, **doing**, **parked** and **done**, carry tags, a description and comments,
and remember when their status changed. Everything is stored as markdown files.

## Contents

1. [Getting started](getting-started.md): install, first run, where data lives
2. [Tasks, tags & search](tasks-and-tags.md): statuses and history, tags, descriptions and comments, filtering the list
3. [Keybindings](keybindings.md): full reference, screen by screen
4. [Command line](cli.md): `tasker list`, `tasker path`
5. [Data format](data-format.md): task files, `tags.md`, editing by hand
6. [Integrations](integrations.md): status bars, syncing between machines, notes apps
7. [FAQ & troubleshooting](faq.md)
8. [Development](development.md): architecture, building, tests, contributing

## Philosophy: lazy

The terminal UI follows the "lazy" TUI school of [lazygit](https://github.com/jesseduffield/lazygit) and [lazyssh](https://github.com/Adembc/lazyssh):

- **Panels, not pages.** The task list and a details panel for the selected task are always visible together.
- **One key per action.** Starting, parking, finishing and reopening a task each take a single keypress.
- **Discoverable.** The footer always shows the keys for the current screen, and `?` opens a menu of every action,
  where `enter` (or the action's own key) runs it. You never have to memorize anything.
- **Safe.** Deleting asks first (`x`, then `y`). Everything is saved immediately, so there's nothing to "save".
- **Zero config.** It works out of the box. The only settings are optional environment variables.
- **Mouse welcome, never required.** The wheel scrolls and a click selects.

The desktop app is the same app for people who'd rather click: the same screens as a conventional window, with a button for
every action, dialogs with Cancel and Save, and the same keys for those who want them.

## Concepts in 30 seconds

| concept | meaning |
| --- | --- |
| **task** | something to do. It has a title, optional tags, a status, a description, comments and a history |
| **status** | `todo` (not started), `doing` (being worked on; any number of tasks can be), `parked` (started, set aside for now), `done` |
| **history** | when the task's status changed, e.g. `todo → doing` on Mon 28 Sep 10:12 |
| **tag** | a label such as `prod` or `support`, shown as a colored chip |
| **description** | free text about the task, like an issue's body |
| **comment** | a timestamped note added over time (like a GitHub/GitLab issue comment); can be edited and deleted |

> **Using this as a GitHub/GitLab wiki:** the pages only link to each other with relative links, so you can copy
> `docs/*.md` into the project wiki as is. Rename `README.md` to `Home.md` for the wiki's start page.
