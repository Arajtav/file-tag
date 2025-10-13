use std::{fmt::Display, str::FromStr};

use rusqlite::{ToSql, types::ToSqlOutput};

use crate::errors::ProgramError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(String);

impl Tag {
    pub fn new(tag: String) -> Result<Self, ProgramError> {
        if tag.is_empty()
            || tag.starts_with("_")
            || tag.ends_with("_")
            || tag.contains("__")
            || tag.chars().any(|c| !c.is_ascii_lowercase() && c != '_')
        {
            Err(ProgramError::InvalidTagName(tag))
        } else {
            Ok(Tag(tag))
        }
    }
}

impl FromStr for Tag {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tag::new(s.to_string())
    }
}

impl ToSql for Tag {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
