# Data format

Everything tasker knows is stored in one folder of plain markdown files. You can read them, edit them, grep them and version them.

```
~/.local/share/tasker/
├── 0001-feature-export-csv.md     ← one file per task
├── 0002-fix-login-500.md
├── 0003-standup.md
└── tags.md                        ← tag colors
```

## Task files

Named `NNNN-slug.md`: the task id, zero-padded, plus a slug of the title. Renaming a task renames its file.

```markdown
# Fix login 500

- id: 2
- status: doing
- tags: prod, support
- created: 2026-09-24T10:12:00+02:00

## Description

Customer X reported it at 10:05.
- check nginx logs
- rollback candidate: v1.4.2

## Comments

### 2026-09-24T10:40:00+02:00 (edited 2026-09-24T10:52:00+02:00)

Found it: nginx timeout is 5s. Root cause confirmed.

### 2026-09-24T11:30:00+02:00

Raised to 30s, monitoring.

## History

- 2026-09-24T10:15:00+02:00 todo -> doing
- 2026-09-24T18:00:00+02:00 doing -> parked
- 2026-09-25T09:30:00+02:00 parked -> doing
```

| part | meaning |
| --- | --- |
| `# title` | first level-1 heading |
| `- id:` | **required**, unique number. New tasks get the highest id + 1 |
| `- status:` | `todo`, `doing`, `parked` or `done` |
| `- tags:` | comma or space separated, may be empty |
| `- created:` | creation time |
| `## Description` | free text about the task, kept as is. Edit it with `m` in the TUI |
| `## Comments` | one `### <created>` heading per comment, optionally followed by `(edited <time>)`, then the comment's text. Oldest first |
| `## History` | one line per status change: `- TIME FROM -> TO`. Oldest first |

Headings inside the description must be `###` or deeper, and inside comments `####` or deeper. A `##` line starts a new section,
and inside Comments a `### <timestamp>` line starts a new comment. The TUI editors adjust headings for you. When editing by hand,
a `### ` line that isn't a timestamp is kept as part of the comment.

Times are stored as RFC 3339 with your UTC offset, so files stay correct across time zones and DST changes.

## Editing by hand

Press `o` on a task to open its file in `$EDITOR`. When you close the editor, tasker reloads everything.
If you edit files outside tasker while it's running, press `R` to reload.

When typing times by hand, all of these are accepted:

```
2026-09-24T10:12:00+02:00      (RFC 3339)
2026-09-24 10:12:00
2026-09-24 10:12
2026-09-24T10:12
```

Times without an offset are read as your local time. On the next save they're rewritten in the full format.

### Fixing mistakes

**Moved a task to done too early, or forgot to.** Press `s`, `p`, `d` or `b` in tasker; each change adds a history line.
To correct the date of a change, open the task with `o` and edit the time on its `## History` line:

```markdown
- 2026-09-25T09:12:00+02:00 doing -> done     ← before
- 2026-09-24 18:30 doing -> done              ← after
```

History lines are sorted by time when read, so the line order doesn't matter.
The `- status:` line decides where the task stands; the history is only the record of how it got there.

### What tasker fixes for you

| situation | what happens on load |
| --- | --- |
| missing `status` | `todo` |
| missing `created` | the time of the first history line, or now |
| a file that can't be read (e.g. no `id`, or a history line it can't parse) | skipped, with a warning in the footer. **The file is not touched** |

Other `.md` files in the folder that aren't tasks (such as a README) are reported as skipped and otherwise ignored.

## `tags.md`

Tag colors, one per line:

```markdown
# Tag colors

Edit freely. Colors: red, orange, gold, …, or #rrggbb.

- prod: red
- support: #33aaff
```

New tags are added automatically. See [tag colors](tasks-and-tags.md#tag-colors).

## Writes are atomic

Task files are written to a temporary file (`.NNNN-slug.md.tmp`) and then renamed into place,
so a crash or power loss never leaves a half-written task file.
