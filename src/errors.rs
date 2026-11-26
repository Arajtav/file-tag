use std::path::PathBuf;

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
    /// There is already an alias of this name.
    #[error("there already is an alias named {0}")]
    AliasExists(Tag),
    /// There is already a tag of this name.
    #[error("there already is a tag named {0}")]
    TagExists(Tag),
    /// Tried to do an operation on inexistent tag.
    #[error("tag {0} doesn't not exist")]
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
    /// Migration failed.
    #[error("migration failed: {0}")]
    MigrationError(#[from] rusqlite_migration::Error),
    /// The path is outside the database directory.
    #[error("outside the database directory ({0}, the database is at {1})")]
    OutsideDb(PathBuf, PathBuf),
}
