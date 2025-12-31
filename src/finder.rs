use std::path::PathBuf;

use directories::ProjectDirs;

use crate::errors::ProgramError;

pub fn find_user_state_dir() -> Option<PathBuf> {
    let project = ProjectDirs::from("com", "arajtav", "file-tag")?;
    let state = project
        .state_dir()
        .unwrap_or_else(|| project.data_local_dir());

    std::fs::create_dir_all(state).ok()?;

    Some(state.to_owned())
}

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
