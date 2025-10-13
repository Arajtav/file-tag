use thiserror::Error;

use crate::tag::Tag;

#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("rusqlite error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("user canceled the action")]
    UserCanceled,
    #[error("invalid tag name {0:?}")]
    InvalidTagName(String),
    #[error("tag {0} already exists")]
    TagExists(Tag),
    #[error("tag {0} doesn not exist")]
    TagNotFound(Tag),
    #[error("cannot merge {0} with itself")]
    SelfMerge(Tag),
    #[error("failed to resolve path {0}")]
    InvalidPath(std::path::PathBuf),
}
