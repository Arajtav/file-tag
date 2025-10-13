use std::path::{PathBuf, absolute};

use crate::errors::ProgramError;

pub fn resolve_path(path: PathBuf) -> Result<String, ProgramError> {
    if let Ok(canonical) = path.canonicalize() {
        return canonical
            .to_str()
            .map(|s| s.to_owned())
            .ok_or(ProgramError::InvalidPath(path));
    }

    // TODO: this isn't reliable, it works fine for untagging and stuff though;
    // I assume tagging inexistent files will be rare.
    if let Ok(absolute) = absolute(&path) {
        return absolute
            .to_str()
            .map(|s| s.to_owned())
            .ok_or(ProgramError::InvalidPath(path));
    }

    Err(ProgramError::InvalidPath(path))
}
