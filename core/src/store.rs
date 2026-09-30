//! The data folder: one `NNNN-slug.md` file per task, and `tags.md`.

use crate::model::{Task, slugify};
use crate::tags::TagColors;
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Tag color assignments; lives next to the task files but isn't a task.
const TAGS_FILE: &str = "tags.md";

/// The data folder where tasks are stored.
#[derive(Debug)]
pub struct Store {
    /// Absolute or relative path of the folder; created on open.
    dir: PathBuf,
}

impl Store {
    /// Uses `$TASKER_DIR`, or the platform data dir (e.g. `~/.local/share/tasker`).
    pub fn open() -> Result<Self> {
        let dir = match std::env::var_os("TASKER_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => dirs::data_dir().context("could not determine the data directory; set TASKER_DIR")?.join("tasker"),
        };
        Self::at(dir)
    }

    /// A store in `dir`, created if missing.
    pub fn at(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self { dir })
    }

    /// The data folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the parsed tasks plus a warning for every file that could not be read.
    pub fn load(&self) -> Result<(Vec<Task>, Vec<String>)> {
        let mut tasks = Vec::new();
        let mut warnings = Vec::new();
        let entries = fs::read_dir(&self.dir).with_context(|| format!("reading {}", self.dir.display()))?;
        for entry in entries {
            let path = entry?.path();
            if !is_task_file(&path) {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let parsed =
                fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| Task::from_markdown(&text));
            match parsed {
                Ok(mut task) => {
                    task.path = Some(path);
                    tasks.push(task);
                }
                Err(e) => warnings.push(format!("{name}: {e}")),
            }
        }
        Ok((tasks, warnings))
    }

    /// Writes atomically and renames the file when the title (slug) changed.
    pub fn save(&self, task: &mut Task) -> Result<()> {
        let name = format!("{:04}-{}.md", task.id, slugify(&task.title));
        let path = self.dir.join(&name);
        // Write to a temporary file, then rename it into place: a crash can never leave a half-written task.
        let tmp = self.dir.join(format!(".{name}.tmp"));
        fs::write(&tmp, task.to_markdown()).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("writing {}", path.display()))?;
        // A new title means a new file name: remove the file saved under the old one.
        if let Some(old) = task.path.replace(path)
            && Some(&old) != task.path.as_ref()
        {
            // The old name only matters if it still exists; a failure here leaves a stale copy, not lost data.
            let _ = fs::remove_file(old);
        }
        Ok(())
    }

    /// Removes the task's file. Refuses anything outside the data folder.
    pub fn delete(&self, task: &Task) -> Result<()> {
        let Some(path) = &task.path else { return Ok(()) };
        if !path.starts_with(&self.dir) {
            bail!("refusing to delete {}: not in {}", path.display(), self.dir.display());
        }
        fs::remove_file(path).with_context(|| format!("deleting {}", path.display()))
    }

    /// Reads `tags.md`; no file means no colors assigned yet.
    pub fn load_tag_colors(&self) -> Result<TagColors> {
        let path = self.dir.join(TAGS_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => Ok(TagColors::parse(&text)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(TagColors::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Writes `tags.md`.
    pub fn save_tag_colors(&self, colors: &TagColors) -> Result<()> {
        let path = self.dir.join(TAGS_FILE);
        fs::write(&path, colors.to_markdown()).with_context(|| format!("writing {}", path.display()))
    }
}

/// Task files are the `.md` files in the folder, except `tags.md`.
fn is_task_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md") && path.file_name().is_some_and(|n| n != TAGS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::now;

    #[test]
    fn save_renames_on_title_change_and_loads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path()).unwrap();
        let mut task = Task::new(3, "First title".into(), now());
        store.save(&mut task).unwrap();
        task.title = "Second title".into();
        store.save(&mut task).unwrap();

        let names: Vec<String> =
            fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["0003-second-title.md"]);

        let (tasks, warnings) = store.load().unwrap();
        assert!(warnings.is_empty());
        assert_eq!(tasks[0].title, "Second title");
    }

    #[test]
    fn unreadable_files_become_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path()).unwrap();
        fs::write(dir.path().join("README.md"), "# not a task").unwrap();
        fs::write(dir.path().join(TAGS_FILE), "- prod: red").unwrap();
        let (tasks, warnings) = store.load().unwrap();
        assert!(tasks.is_empty());
        assert_eq!(warnings.len(), 1, "tags.md is not a task: {warnings:?}");
    }
}
