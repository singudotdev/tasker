# Keybindings

You don't need to memorize these:

- The **footer** always shows every key for the current screen. **No key works that isn't listed there.** Hints are mnemonic:
  the key is in parentheses, inside the word when it fits (`(n)ew`, `(e)dit`, `new (c)omment`) and in front otherwise
  (`(x) delete`, `(esc) back`). Upper and lower case are different keys: `(R)eload` isn't `r`.
  The footer wraps onto more lines in narrow terminals, and temporary messages replace it for 5 seconds.
- **`?`** opens a menu with a description of every action on the current screen. Move with `↑↓`/`jk` and press `enter` to run the
  highlighted action, or press the action's own key. `esc` closes it.

The footer and the `?` menu are generated from the same list of bindings (`core/src/keys.rs`), so they always match this page.

**The desktop app (`tasker-gui`)** has a button for every action, and the same keys work there too: every button's tooltip names
its key, and `?` opens the list of shortcuts. It shows no footer. One difference to keep in mind: a key typed while the
search box isn't selected runs its action (e.g. `d` marks the selected task done), just like in the terminal.

In the desktop app's text fields and editor, `ctrl+a` selects all, `ctrl+c` / `ctrl+x` / `ctrl+v` copy, cut and paste with the
system clipboard (`cmd` on macOS), and `shift` with the arrows, `home` or `end` selects text. Typing or pasting replaces
the selection. For the mouse, see [Mouse](#mouse). In the terminal, copy and paste are the terminal's own.

## Task list

| key | action |
| --- | --- |
| `↑` `↓` / `j` `k` | move |
| `n` | **new** task, as `todo` |
| `s` | **start**: move the selected task to `doing` |
| `p` | **park**: move the selected task to `parked` (started, set aside for now) |
| `d` | **done**: move the selected task to `done` |
| `b` | **back to todo**: move the selected task to `todo` |
| `e` | **edit** title and tags |
| `v` | **view** the task like an issue: description, comments and history (see below) |
| `m` | edit the task's **description** |
| `c` | add a **comment** to the selected task |
| `o` | **open** the task's markdown file in `$EDITOR` |
| `x` | **delete** the selected task (asks `y` to confirm) |
| `/` | **search** |
| `esc` | clear the search (only while a search filter is active; the footer shows it then) |
| `f` | **filter** the list: active → all (incl. done) → done only |
| `t` | **tags**: rename, recolor or delete a tag on every task |
| `R` | **reload** all files from disk |
| `?` | keys menu: every action, runnable |
| `q` | **quit** |

## Search box (`/`)

| key | action |
| --- | --- |
| type / `backspace` / `delete` | edit the query at the cursor; the list filters live |
| `←` `→` / `home` `end` | move the cursor in the query |
| `↑` `↓` | move through the results |
| `enter` | keep the filter, back to the list |
| `esc` | clear the filter, back to the list |

## Create / edit form (`n`, `e`)

| key | action |
| --- | --- |
| type / `backspace` / `delete` | edit the field at the cursor |
| `←` `→` / `home` `end` | move the cursor within the field |
| `tab` / `↑` `↓` | switch between the title and tags fields |
| `enter` | save |
| `esc` | cancel |

## Task view (`v`)

The selected task shown like a GitHub/GitLab issue: status, tags and dates, the description, the status history, then the comments (oldest first).

| key | action |
| --- | --- |
| `↑` `↓` / `j` `k` | select a comment |
| `c` | new comment |
| `e` | edit the selected comment |
| `x` | delete the selected comment (asks `y` to confirm) |
| `m` | edit the description |
| `s` / `p` / `d` / `b` | move the task to doing / parked / done / todo |
| `?` | keys menu for this view |
| `esc` | back to the task list |

## Text editor (`m`, `c`, `e` in the task view)

The same multi-line editor is used for the description, new comments and editing comments. It opens over the current screen.

| key | action |
| --- | --- |
| type | insert text (`tab` inserts two spaces). Long lines wrap on screen and are saved as one line |
| `enter` | new line |
| arrows, `home`, `end` | move |
| `backspace` / `delete` | delete a character; at the start or end of a line, join the lines |
| `esc` | **save** and close |
| `ctrl+c` | **discard** the changes and close (terminal only: the desktop app copies, and has a Discard changes button) |

`●` in the title means there are unsaved changes. `esc` always saves, so closing never loses text.

## Delete confirmation (`x`)

Used for tasks, comments and tags.

| key | action |
| --- | --- |
| `y` | delete (`enter` does **not** confirm a delete) |
| any other key | cancel |

## Tags (`t`)

Every tag in use, alphabetically, with its color and how many tasks in each status carry it.

| key | action |
| --- | --- |
| `↑` `↓` / `j` `k` | select a tag |
| `r` | **rename** the tag on every task. Renaming to a tag that's already in use **merges** the two, keeping that tag's color |
| `c` / `C` | next / previous palette **color** |
| `x` | **delete** the tag from every task (asks `y` to confirm). The tasks stay |
| `?` | keys menu for this screen |
| `esc` | back to the task list |

In the rename dialog, edit the name (`←` `→` / `home` `end` move the cursor, `backspace` / `delete` erase), then `enter` to rename or `esc` to cancel. The name is one word; it's saved in lowercase, without `#`.

## Keys menu (`?`)

| key | action |
| --- | --- |
| `↑` `↓` / `j` `k` | move |
| `enter` | run the highlighted action |
| an action's key (e.g. `s`) | run that action directly |
| `esc` | close |

## Mouse

**Terminal**

| action | effect |
| --- | --- |
| wheel | move the selection in the task list, task view, tags screen and keys menu |
| left click | select a task in the list |

tasker captures the mouse. To select text in the terminal, hold `shift` (on macOS: `fn` or `option`, depending on the terminal) while dragging.

**Desktop app**: every action has a button; the mouse also does the following.

| action | effect |
| --- | --- |
| double-click a task | open it |
| click a `?` shortcut | run it |
| back button | leave the task view or the tags screen |
| click in text | place the cursor |
| drag, `shift`+click | select |
| double-click / triple-click in text | select a word / the line; dragging on extends by words / lines |
| middle-click in text (Linux) | paste the primary selection (the text last selected, in any app) |
