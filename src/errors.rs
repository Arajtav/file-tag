use thiserror::Error;

use crate::tag::Tag;

/// All the errors returned by the cli or the backend.
#[derive(Error, Debug)]
pub enum ProgramError {
    /// Rusqlite/sqlite error.
    #[error("rusqlite error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    /// Action cancelled by the user.
    #[error("action cancelled by the user")]
    UserCanceled,
    /// Disallowed tag name.
    #[error("invalid tag name {0:?}")]
    InvalidTagName(String),
    /// Tried to overwrite a tag.
    #[error("tag {0} already exists")]
    TagExists(Tag),
    /// Tried to do an operation on inexistent tag.
    #[error("tag {0} doesn not exist")]
    TagNotFound(Tag),
    /// Tried to merge a tag with itself.
    #[error("cannot merge {0} with itself")]
    SelfMerge(Tag),
    /// Failed to resolve file path.
    #[error("failed to resolve path {0}")]
    InvalidPath(std::path::PathBuf),
    /// Failed to get the database.
    #[error("failed to get the database")]
    NoDB,
}
