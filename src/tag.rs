use std::{fmt::Display, str::FromStr};

use rusqlite::{ToSql, types::ToSqlOutput};

use crate::errors::ProgramError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(String);

impl Tag {
    pub fn new(original: &str) -> Result<Self, ProgramError> {
        /*
         * make lowercase.
         * spaces are converted to underscores.
         * must start with a letter.
         * only ascii letters, digits, hyphens, underscores, and normal brackets are allowed.
         * hyphens must be surrounded by letters or digits.
         * multiple consecutive underscores (spaces) are collapsed into one.
         * result is trimmed from underscores (spaces).
         * result cannot be empty.
         */
        let mut last = '_'; // thanks to this, there is no need to check for leading underscores or hyphens.
        let mut acc = String::with_capacity(original.len());
        for mut char in original.to_lowercase().chars() {
            if char == ' ' {
                char = '_';
            }

            if !(char.is_ascii_alphanumeric()
                || char == '('
                || char == ')'
                || char == '_'
                || char == '-')
                || (last == '-' && !char.is_alphanumeric())
                || (char == '-' && !last.is_alphanumeric())
            {
                return Err(ProgramError::InvalidTagName(original.to_owned()));
            }

            if !(char == '_' && last == '_') {
                acc.push(char);
            }
            last = char;
        }

        let tag = acc.trim_end_matches('_').to_owned();

        if !tag.is_empty()
            && (tag.as_bytes()[0] as char).is_ascii_alphabetic()
            && !tag.ends_with('-')
        {
            Ok(Tag(tag))
        } else {
            Err(ProgramError::InvalidTagName(original.to_owned()))
        }
    }
}

impl FromStr for Tag {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
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

impl<T> PartialEq<T> for Tag
where
    T: AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        self.0 == other.as_ref()
    }
}

#[test]
fn test_tag() {
    assert_eq!(Tag::new("tag").unwrap(), "tag");
    assert_eq!(Tag::new(" tag").unwrap(), "tag");
    assert_eq!(Tag::new("tag ").unwrap(), "tag");
    assert_eq!(Tag::new("__ tag ").unwrap(), "tag");
    assert_eq!(Tag::new("ta g").unwrap(), "ta_g");
    assert_eq!(Tag::new("Tag").unwrap(), "tag");
    assert_eq!(Tag::new("tag-tag").unwrap(), "tag-tag");
    assert_eq!(Tag::new("tag 4").unwrap(), "tag_4");
    assert_eq!(Tag::new("tag (tag)").unwrap(), "tag_(tag)");
    for tag in [
        "", "_", "Ą", "ð", "(test", "234a", "-", "-tag", "tag--tag", "tag-", "tag_-t", "tag_-_2",
    ] {
        assert!(matches!(
            Tag::new(tag).unwrap_err(),
            ProgramError::InvalidTagName(_)
        ));
    }
}
