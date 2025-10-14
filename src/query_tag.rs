use std::str::FromStr;

use crate::{errors::ProgramError, tag::Tag};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryTag {
    Required(Tag),
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
