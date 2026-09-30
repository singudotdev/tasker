# Tasks, tags & search

## Tasks

Every task is **todo**, **doing**, **parked** or **done**. Any number of tasks can be in doing at once.
**Parked** is for work you've started and set aside for now: blocked, waiting on someone, or paused for later.

| action | key | notes |
| --- | --- | --- |
| create | `n` | starts as `todo` |
| start | `s` | moves it to `doing` |
| finish | `d` | moves it to `done` |
| park | `p` | moves it to `parked` |
| back to todo | `b` | moves it to `todo`, e.g. to reopen a `done` one |
| edit title and tags | `e` | |
| view description, comments and history | `v` | the task as an issue, see [below](#description-and-comments) |
| edit the description | `m` | multi-line editor in the TUI: `esc` saves, `ctrl+c` discards |
| add a comment | `c` | from the list or the task view |
| open the file in your editor | `o` | to change anything at once, or fix a date, see [Data format](data-format.md) |
| delete | `x` | asks for confirmation (`y`), then removes the file |

`s`, `p`, `d` and `b` also work in the task view. Any status can move to any other, e.g. `d` straight from `todo`.

### Status history

Every status change is recorded with its date and time, e.g. `Mon 28 Sep 2026 10:12  todo → doing`.

- The list's **since** column shows how long ago each task's status last changed (`0m`, `5m`, `3h`, `2d`, `6w`).
- The details panel shows the status, since when, the creation date and the latest changes.
- The task view (`v`) shows the whole history.

It's stored in the task file's `## History` section; see [Data format](data-format.md).

### The create / edit form

```
┌ new task ────────────────────────────────────────────────────┐
│title  PROD fix login 500                                     │
│tags   prod, support                                          │
│                                                              │
│known:  backend   feature   prod   support                    │
└ (tab/↑↓) switch field  (enter) ok  (esc) cancel ─────────────┘
```

- `tab` (or `↑` `↓`) switches between **title** and **tags**.
- `←` `→`, `home` and `end` move the cursor, so you can fix a typo anywhere; `backspace` and `delete` erase around it.
- `enter` saves. The title can't be empty.
- The bottom line shows the tags you've already used, so you can reuse the same names.

### Details panel

The panel next to the list (or below it, in tall narrow terminals) shows the selected task:
its tags, status and since when, when it was created, the latest status changes,
the description, and the latest comment. Press `v` to see everything, `m` to edit the description, or `c` to comment. In small terminals the panel is hidden and the list uses the whole width.

### Description and comments

Tasks work like issues on GitHub or GitLab:

- **Description:** what the task is about. There's one per task, and you can edit it any time with `m`.
- **Comments:** timestamped entries you add as work goes on ("found the cause", "waiting for review", "deployed").
  Each one records when it was written and, if you change it later, when it was **edited**.

`v` opens the **task view**:

```
┌ ▶ #2 Fix login 500 ──────────────────────────────────────────────────────────────┐
│ doing   prod   support   for 2h · created 2026-09-24
│
│ Description  (m) edit
│ Customer X reported it at 10:05. Only affects SSO users.
│
│ History  (s)tart  (p)ark  (d)one  (b)ack to todo
│ Thu 24 Sep 2026 10:15  todo → ▶ doing
│
│ Comments (2)  (c) new  (e)dit  (x) delete  (↑↓/jk) select
│
│   Thu 24 Sep 2026 10:40 · edited 10:52
│ │ Found it: nginx timeout is 5s. Root cause confirmed.
│
│ › Thu 24 Sep 2026 11:30
│ │ Raised to 30s, monitoring.
└──────────────────────────────────────────────────────────────────────────────────┘
```

| action | where | key |
| --- | --- | --- |
| add a comment | task list or task view | `c` |
| edit a comment | task view | select it with `↑↓`/`jk`, then `e` |
| delete a comment | task view | select it, then `x`, then `y` to confirm |
| edit the description | task list or task view | `m` |
| change the status | task list or task view | `s` doing, `p` parked, `d` done, `b` todo |

Saving an empty new comment adds nothing. Emptying an existing comment leaves it unchanged; delete comments with `x`.
Markdown headings inside descriptions and comments are moved down a level when saved (`##` → `###` in a description,
`##`/`###` → `####` in a comment), because those levels mark sections and comments in the task file.

### Task list order

1. `doing` tasks
2. `todo` tasks
3. `parked` tasks
4. `done` tasks (when shown)

Within each group, the most recently changed task comes first. Status icons: `▶` doing, `·` todo, `‖` parked, `✓` done.
The header counts the tasks in each status.

## Tags

Tags group related work, e.g. `prod`, `support`, `meeting`, `feature`, or a client or project name.

- Separate tags with commas or spaces: `prod, support`, `prod support` and `#prod #support` all mean the same.
- Tags are saved **in lowercase, without `#`**, and duplicates are removed.
- A task can have any number of tags, or none.
- Tags are shown **everywhere a task appears**: the task list, the details panel, the task view, dialogs and `tasker list`.

### Managing tags

Press `t` in the task list to open the **tags screen**: every tag, its color, and how many tasks in each status use it.

```
┌ tags · 4 ──────────────────────────────────────────┐
│  tag          color    doing todo  parked done     │
│›  backend     indigo   1     1     0      0        │
│   prod        red      1     0     1      2        │
│   support     sky      0     2     0      3        │
└────────────────────────────────────────────────────┘
```

| key | action |
| --- | --- |
| `r` | **rename** the tag on every task, done ones included. Renaming to an existing tag **merges** them (e.g. `ops` → `devops`), and the merged tag keeps that tag's color |
| `c` / `C` | step to the next / previous palette color; the change is saved to `tags.md` at once |
| `x` | **delete** the tag from every task (confirm with `y`). The tasks themselves stay |

To change the tags of **one** task, select it in the list and press `e`.

### Tag colors

Each tag is shown as a colored chip, like in Proxmox VE.

- A new tag gets a **random color from the colors used least so far**. The 16 palette colors are all used once before any repeats.
- Once assigned, **a tag keeps its color**. Adding tags or deleting tasks doesn't reshuffle them.
- The text on the chip is black or white, whichever is easier to read on that background.

The assignments are stored in `tags.md` in your data folder, and you can edit it:

```markdown
# Tag colors

- prod: red
- support: #33aaff
- meeting: steel
```

Available palette names: `red`, `orange`, `gold`, `green`, `emerald`, `teal`, `blue`, `indigo`, `purple`, `magenta`, `pink`, `brown`,
`steel`, `olive`, `salmon`, `sky`. You can also use any hex color `#rrggbb`.

Palette colors use the 256-color range and work in any modern terminal. Hex colors need a truecolor terminal.
After editing the file, press `R` in tasker to reload. To pick a palette color without editing the file, use `c` / `C` on the [tags screen](#managing-tags).

## Search

Press `/` to open the search box above the task list. The list filters as you type.

| query | matches |
| --- | --- |
| `login` | titles or tags containing "login" |
| `#pro` | tasks with a tag **starting with** "pro" (tags only) |
| `login #prod` | **all** terms must match: "login" in the title or tags, **and** a `prod…` tag |

Matching ignores upper/lower case.

| key | action |
| --- | --- |
| `↑` / `↓` | move through the results while typing |
| `enter` | keep the filter and go back to the list, where all task keys work on the filtered list |
| `esc` | clear the search, either while typing or later from the list |

While a filter is active, the line above the list shows the query and the number of matches.

## Done tasks

Press `f` in the task list to cycle what's shown:

| view | list title |
| --- | --- |
| active (default) | `tasks · 3 done hidden (f)` |
| all, including done | `tasks · incl. done` |
| done only | `done tasks · 3` |

This combines with search, e.g. `f` `f` then `/` `#prod` finds finished prod work.
To reopen a done task, select it and press `s`.
