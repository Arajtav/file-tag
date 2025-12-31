use std::str::FromStr;

use crate::{errors::ProgramError, tag::Tag};

/// `QueryTag` type mainly to deserialize the query.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryTag {
    /// Tag to be searched for.
    Required(Tag),
    /// Tag to be excluded from the result.
    Forbidden(Tag),
}

impl FromStr for QueryTag {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(rest) = s.strip_prefix('-') {
            Self::Forbidden(Tag::new(rest)?)
        } else {
            Self::Required(Tag::new(s)?)
        })
    }
}
