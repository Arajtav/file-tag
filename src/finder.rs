use std::{collections::VecDeque, path::PathBuf};

use directories::ProjectDirs;

use crate::errors::ProgramError;

/// Tries to find user's state dir.
pub fn find_user_state_dir() -> Option<PathBuf> {
    let project = ProjectDirs::from("com", "arajtav", "file-tag")?;
    let state = project
        .state_dir()
        .unwrap_or_else(|| project.data_local_dir());

    std::fs::create_dir_all(state).ok()?;

    Some(state.to_owned())
}

/// Tries to find a database to use.
pub fn find_db() -> Result<PathBuf, ProgramError> {
    if let Some(db) = std::env::current_dir()
        .expect("Failed to get cwd") // not ProgramError
        .ancestors()
        .map(|p| p.join("file-tag.sqlite"))
        .find(|p| p.exists())
    {
        Ok(db)
    } else {
        let mut db = find_user_state_dir().ok_or(ProgramError::NoDB)?;
        db.push("db.sqlite");
        Ok(db)
    }
}

/// File scanner for lazily listing files.
pub struct FileScanner {
    files: VecDeque<PathBuf>,
    entry_points: VecDeque<PathBuf>,
}

impl FileScanner {
    /// Create a new scanner from `entry`.
    pub fn new(entry: PathBuf) -> Self {
        Self {
            files: VecDeque::new(),
            entry_points: vec![entry].into(),
        }
    }

    /// Does a single scan.
    fn scan(&mut self) {
        let Some(entry) = self.entry_points.pop_front() else {
            return;
        };

        let dir = match std::fs::read_dir(&entry) {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("Error reading directory {}: {err}", entry.display());
                return;
            }
        };

        for e in dir.into_iter().filter_map(Result::ok) {
            if let Ok(ft) = e.file_type() {
                if ft.is_symlink() {
                    continue;
                }

                if ft.is_file() {
                    self.files.push_back(e.path());
                } else {
                    self.entry_points.push_back(e.path());
                }
            }
        }
    }
}

impl Iterator for FileScanner {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        while self.files.is_empty() && !self.entry_points.is_empty() {
            self.scan();
        }
        self.files.pop_front()
    }
}
