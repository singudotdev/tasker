//! The command line: which command to run, with its options.

use crate::app::App;
use crate::model::{Status, Task, parse_tags};
use crate::platform;
use crate::store::Store;
use anyhow::{Context, Result, anyhow, bail};
use std::fmt::Write;

/// The binary running the command line: `tasker` (the terminal UI) or `tasker-gui` (the desktop app).
#[derive(Debug, Clone, Copy)]
pub struct Program {
    /// The command's name.
    pub name: &'static str,
    /// What running it without arguments does, for the usage text: "open the TUI".
    pub open: &'static str,
}

impl Program {
    /// Parses the arguments after the program name and runs the command; `open` runs the interactive UI.
    /// A command-line error comes with the usage text.
    pub fn main(self, args: impl IntoIterator<Item = String>, open: impl FnOnce(App) -> Result<()>) -> Result<()> {
        let command = Command::parse(args).map_err(|e| anyhow!("{e:#}\n\n{}", usage(self)))?;
        command.run(self, open)
    }
}

/// Printed by `--help` and after a command-line error. The status names come from `Status`,
/// so they're only written there.
pub fn usage(program: Program) -> String {
    let name = program.name;
    let pad = " ".repeat(name.len());
    format!(
        "{name} — todo tracker backed by markdown files

usage:
  {name}           {open}
  {name} list [{statuses}|all] [--tag TAG]
  {pad}           print tasks as markdown, one section per status
  {pad}           (default: {default}; several statuses can be given)
  {name} path      print the data directory
  {name} --version print the version

Data lives in $TASKER_DIR, or the platform data dir (e.g. ~/.local/share/tasker).
The terminal UI (tasker) and the desktop app (tasker-gui) share it.",
        open = program.open,
        statuses = Status::names("|"),
        default = DEFAULT_LIST.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
    )
}

/// What `tasker list` shows unless statuses are given: everything that isn't done.
const DEFAULT_LIST: &[Status] = &[Status::Doing, Status::Todo, Status::Parked];

/// What to do, as chosen on the command line.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the interactive UI (no arguments).
    Open,
    /// `list`: print tasks as markdown.
    List(ListArgs),
    /// `path`: print the data folder.
    Path,
    /// `--version` / `-V`.
    Version,
    /// `--help` / `-h`.
    Help,
}

/// Options of `tasker list`.
#[derive(Debug, PartialEq, Eq)]
pub struct ListArgs {
    /// Statuses to show, in list order.
    pub statuses: Vec<Status>,
    /// Only tasks with this tag.
    pub tag: Option<String>,
}

impl Command {
    /// Parses the arguments after the program name.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut args = args.into_iter();
        let command = match args.next().as_deref() {
            None => Command::Open,
            Some("list") => Command::List(ListArgs::parse(args.by_ref())?),
            Some("path") => Command::Path,
            Some("-V" | "--version") => Command::Version,
            Some("-h" | "--help") => Command::Help,
            Some(other) => bail!("unknown command `{other}`"),
        };
        if let Some(extra) = args.next() {
            bail!("unexpected argument `{extra}`");
        }
        Ok(command)
    }

    /// Runs the command; `open` runs the interactive UI. Everything except `--version` and `--help`
    /// refuses to run as root.
    pub fn run(self, program: Program, open: impl FnOnce(App) -> Result<()>) -> Result<()> {
        match self {
            Command::Version => {
                println!("{} {}", program.name, env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Command::Help => {
                println!("{}", usage(program));
                return Ok(());
            }
            _ => platform::ensure_not_root()?,
        }
        let store = Store::open()?;
        match self {
            Command::Open => open(App::new(store)?),
            Command::List(args) => {
                let (tasks, _) = store.load()?;
                print!("{}", list_markdown(&tasks, &args));
                Ok(())
            }
            Command::Path => {
                println!("{}", store.dir().display());
                Ok(())
            }
            // Handled above, before touching the data folder.
            Command::Version | Command::Help => Ok(()),
        }
    }
}

impl ListArgs {
    /// Parses the options after `list`.
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self> {
        let mut statuses = Vec::new();
        let mut tag = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--tag" => {
                    let value = args.next().context("--tag needs a value")?;
                    tag = parse_tags(&value).into_iter().next();
                }
                "all" => statuses.extend(Status::ALL),
                // Anything else must be a status name.
                other => match Status::parse(other) {
                    Some(status) => statuses.push(status),
                    None => bail!("unknown list option `{other}`"),
                },
            }
        }
        if statuses.is_empty() {
            statuses.extend(DEFAULT_LIST);
        }
        // List order, without repeats.
        statuses.sort_by_key(|s| s.position());
        statuses.dedup();
        Ok(Self { statuses, tag })
    }
}

/// One `## status` section per status that has tasks, each task as
/// `- #id title #tag …`, most recently changed first.
fn list_markdown(tasks: &[Task], args: &ListArgs) -> String {
    let mut out = String::new();
    for &status in &args.statuses {
        let mut matching: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.status == status && args.tag.as_ref().is_none_or(|tag| t.tags.contains(tag)))
            .collect();
        if matching.is_empty() {
            continue;
        }
        matching.sort_by(|a, b| b.status_since().cmp(&a.status_since()).then(b.id.cmp(&a.id)));
        if !out.is_empty() {
            out.push('\n');
        }
        writeln!(out, "## {}\n", status.as_str()).expect("writing to a String cannot fail");
        for task in matching {
            write!(out, "- #{} {}", task.id, task.title).expect("writing to a String cannot fail");
            for tag in &task.tags {
                write!(out, " #{tag}").expect("writing to a String cannot fail");
            }
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::now;

    fn parse(args: &[&str]) -> Result<Command> {
        Command::parse(args.iter().map(|s| (*s).to_string()))
    }

    #[test]
    fn commands() {
        assert_eq!(parse(&[]).unwrap(), Command::Open);
        assert_eq!(parse(&["path"]).unwrap(), Command::Path);
        assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&["path", "extra"]).is_err());
    }

    #[test]
    fn list_options() {
        let list = |args: &[&str]| match parse(args).unwrap() {
            Command::List(args) => args,
            other => panic!("expected a list command, got {other:?}"),
        };
        assert_eq!(list(&["list"]).statuses, [Status::Doing, Status::Todo, Status::Parked]);
        assert_eq!(
            list(&["list", "done", "doing", "--tag", "#Prod", "done"]),
            ListArgs { statuses: vec![Status::Doing, Status::Done], tag: Some("prod".into()) }
        );
        assert_eq!(list(&["list", "all"]).statuses, Status::ALL);
        assert!(parse(&["list", "paused"]).is_err());
        assert!(parse(&["list", "--tag"]).is_err());
    }

    #[test]
    fn list_output() {
        let mut fix = Task::new(1, "Fix login".into(), now());
        fix.tags = vec!["prod".into(), "support".into()];
        fix.set_status(Status::Doing, now());
        let docs = Task::new(2, "Write docs".into(), now());
        let mut old = Task::new(3, "Old thing".into(), now());
        old.set_status(Status::Done, now());
        let tasks = [fix, docs, old];

        let all = ListArgs { statuses: Status::ALL.to_vec(), tag: None };
        assert_eq!(
            list_markdown(&tasks, &all),
            "## doing\n\n- #1 Fix login #prod #support\n\n## todo\n\n- #2 Write docs\n\n## done\n\n- #3 Old thing\n"
        );
        let prod = ListArgs { statuses: Status::ALL.to_vec(), tag: Some("prod".into()) };
        assert_eq!(list_markdown(&tasks, &prod), "## doing\n\n- #1 Fix login #prod #support\n");
    }
}
