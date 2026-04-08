use std::path::{PathBuf, absolute};

use crate::errors::ProgramError;

/// Tries to resolve a path, canonicalize, etc.
pub fn resolve_path(path: PathBuf) -> Result<String, ProgramError> {
    let Ok(resolved) = path.canonicalize().or_else(|_| absolute(&path)) else {
        return Err(ProgramError::InvalidPath(path));
    };

    resolved
        .into_os_string()
        .into_string()
        .map_err(|_| ProgramError::InvalidPath(path))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use assert_fs::{TempDir, prelude::*};

    use super::*;

    #[test]
    fn paths_normalize_same() {
        let temp = TempDir::new().unwrap();

        let dir = temp.child("dir");
        dir.create_dir_all().unwrap();

        let p1 = dir.child("../dir").path().to_path_buf();
        let p2 = dir.path().to_path_buf();

        let r1 = resolve_path(p1).unwrap();
        let r2 = resolve_path(p2).unwrap();

        assert_eq!(Path::new(&r1), Path::new(&r2));
    }

    #[test]
    fn deleted_file_tries_to_normalize() {
        let temp = TempDir::new().unwrap();

        let file = temp.child("file.txt");
        file.touch().unwrap();
        let path = file.path().to_path_buf();

        std::fs::remove_file(&path).unwrap();

        let resolved = resolve_path(path.clone()).unwrap();

        assert_eq!(
            resolved,
            path.canonicalize()
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
        );
    }

    #[test]
    fn nested_relative_paths_normalize() {
        let temp = TempDir::new().unwrap();
        let nested = temp.child("a/b/c");
        nested.create_dir_all().unwrap();

        let p1 = nested.path().join("../../b/./c");
        let p2 = nested.path().to_path_buf();

        let r1 = resolve_path(p1).unwrap();
        let r2 = resolve_path(p2).unwrap();

        assert_eq!(Path::new(&r1), Path::new(&r2));
    }
}
