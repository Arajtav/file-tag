use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::{errors::ProgramError, migrations::MIGRATIONS, tag::Tag};

/// App struct.
/// Encapsulates all the backend logic.
pub struct App {
    /// Database connection.
    conn: Connection,
}

/// A function to call to get user confirmation if needed.
pub type Confirm = fn() -> bool;

impl App {
    fn init_db(conn: &mut Connection) -> Result<(), ProgramError> {
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(ProgramError::RusqliteError)?;

        MIGRATIONS
            .to_latest(conn)
            .map_err(ProgramError::MigrationError)
    }

    /// Opens a database connection from `db_path`.
    /// Creates and initializes a new database if needed.
    pub fn new(db_path: &Path) -> Result<Self, ProgramError> {
        let mut conn = Connection::open(db_path).map_err(ProgramError::RusqliteError)?;
        Self::init_db(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens a database connection in memory.
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, ProgramError> {
        let mut conn = Connection::open_in_memory().map_err(ProgramError::RusqliteError)?;
        Self::init_db(&mut conn)?;
        Ok(Self { conn })
    }

    /// Counts how many files is the `tag` on.
    fn count_uses(tx: &Transaction, tag: &Tag) -> Result<i64, ProgramError> {
        let tag_exists: bool = tx
            .prepare("SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)")
            .map_err(ProgramError::RusqliteError)?
            .query_row([tag], |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?;

        if !tag_exists {
            return Err(ProgramError::TagNotFound(tag.to_owned()));
        }

        let count = tx
            .prepare("SELECT COUNT(*) FROM entry_tags WHERE tag = ?1")
            .map_err(ProgramError::RusqliteError)?
            .query_row([tag], |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?;

        Ok(count)
    }

    /// Creates a new tag.
    /// Returns false if the tag already exists.
    pub fn create_tag(&mut self, tag: &Tag) -> Result<bool, ProgramError> {
        Ok(self
            .conn
            .execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
            .map_err(ProgramError::RusqliteError)?
            != 0)
    }

    /// Removes an existing tag.
    pub fn remove_tag(&mut self, tag: &Tag, confirm: Confirm) -> Result<(), ProgramError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        let files: Vec<String> = tx
            .prepare("SELECT DISTINCT entry FROM entry_tags WHERE tag = ?1")
            .map_err(ProgramError::RusqliteError)?
            .query_map([tag], |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgramError::RusqliteError)?;

        if !files.is_empty() {
            println!("{} files are tagged with that tag", files.len());

            if !confirm() {
                return Err(ProgramError::UserCanceled);
            }
        }

        tx.execute("DELETE FROM tags WHERE name = ?1", [tag])
            .map_err(ProgramError::RusqliteError)?;
        for file in files {
            tx.execute("DELETE FROM entries WHERE path = ?1 AND NOT EXISTS (SELECT 1 FROM entry_tags WHERE entry = ?1)", [file])
                .map_err(ProgramError::RusqliteError)?;
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok(())
    }

    /// Renames a tag.
    pub fn rename_tag(
        &mut self,
        old: &Tag,
        new: &Tag,
        confirm: Confirm,
    ) -> Result<(), ProgramError> {
        if old == new {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        if !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [old],
                |row| row.get::<_, bool>(0),
            )
            .map_err(ProgramError::RusqliteError)?
        {
            return Err(ProgramError::TagNotFound(old.to_owned()));
        }

        if tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [new],
                |row| row.get(0),
            )
            .map_err(ProgramError::RusqliteError)?
        {
            return Err(ProgramError::TagNotFound(new.to_owned()));
        }

        if !confirm() {
            return Err(ProgramError::UserCanceled);
        }

        tx.execute("UPDATE tags SET name = ?1 WHERE name = ?2", [new, old])
            .map_err(ProgramError::RusqliteError)?;

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok(())
    }

    /// Merges 2 tags.
    /// If `new` is `None`, the result is saved under the name of `tag_a`.
    pub fn merge_tags(
        &mut self,
        tag_a: &Tag,
        tag_b: &Tag,
        new: Option<&Tag>,
        confirm: Confirm,
    ) -> Result<(), ProgramError> {
        if tag_a == tag_b {
            return Err(ProgramError::SelfMerge(tag_a.to_owned()));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        println!(
            "{} files are tagged with {tag_a}",
            App::count_uses(&tx, tag_a)?
        );

        println!(
            "{} files are tagged with {tag_b}",
            App::count_uses(&tx, tag_b)?
        );

        if let Some(new) = new
            && new != tag_a
            && new != tag_b
            && tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                    [new],
                    |row| row.get(0),
                )
                .map_err(ProgramError::RusqliteError)?
        {
            return Err(ProgramError::TagExists(new.to_owned()));
        }

        if !confirm() {
            return Err(ProgramError::UserCanceled);
        }

        tx.execute(
            "UPDATE OR IGNORE entry_tags SET tag = ?1 WHERE tag = ?2",
            [tag_a, tag_b],
        )
        .map_err(ProgramError::RusqliteError)?;

        tx.execute("DELETE FROM tags WHERE name = ?1", [tag_b])
            .map_err(ProgramError::RusqliteError)?;

        if let Some(new) = new {
            tx.execute("UPDATE tags SET name = ?1 WHERE name = ?2", [new, tag_a])
                .map_err(ProgramError::RusqliteError)?;
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok(())
    }

    /// Returns the list of tags.
    pub fn tags(&self, entry: Option<&str>) -> Result<Vec<Tag>, ProgramError> {
        self.conn
            .prepare(
                "SELECT name FROM tags
                 WHERE ?1 IS NULL
                    OR name IN (SELECT tag FROM entry_tags WHERE entry = ?1)",
            )
            .map_err(ProgramError::RusqliteError)?
            .query_map([entry], |row| row.get::<_, String>(0))
            .map_err(ProgramError::RusqliteError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgramError::RusqliteError)
            .map(|v| v.into_iter().map(Tag::new_unchecked).collect())
    }

    /// Like `tags` but the number of times each tag is used is returned along the tag.
    pub fn tags_counted(&self, entry: Option<&str>) -> Result<Vec<(Tag, i64)>, ProgramError> {
        self.conn
            .prepare(
                "
                    SELECT t.name, COUNT(et.entry)
                    FROM tags t
                    LEFT JOIN entry_tags et ON et.tag = t.name
                    WHERE ?1 IS NULL
                    OR t.name IN (SELECT tag FROM entry_tags WHERE entry = ?1)
                    GROUP BY t.name
                 ",
            )
            .map_err(ProgramError::RusqliteError)?
            .query_map([entry], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(ProgramError::RusqliteError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgramError::RusqliteError)
            .map(|v| {
                v.into_iter()
                    .map(|(name, count)| (Tag::new_unchecked(name), count))
                    .collect()
            })
    }

    /// Returns tags and count of shared entries with `tag`.
    pub fn tag_associated(
        &self,
        tag: &Tag,
        limit: Option<i64>,
    ) -> Result<Vec<(Tag, i64)>, ProgramError> {
        self.conn
            .prepare(
                "
                    SELECT et2.tag, COUNT(et2.entry) AS shared
                    FROM entry_tags et1
                    JOIN entry_tags et2 ON et2.entry = et1.entry
                    WHERE et1.tag = ?1
                        AND et2.tag != ?1
                    GROUP BY et2.tag
                    ORDER BY shared DESC, et2.tag ASC
                    LIMIT ?2
                ",
            )
            .map_err(ProgramError::RusqliteError)?
            .query_map(rusqlite::params![tag, limit.unwrap_or(-1)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(ProgramError::RusqliteError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgramError::RusqliteError)
            .map(|v| {
                v.into_iter()
                    .map(|(name, count)| (Tag::new_unchecked(name), count))
                    .collect()
            })
    }

    /// Returns whether the file is in the database.
    pub fn is_in_db(&self, path: &str) -> Result<bool, ProgramError> {
        Ok(self
            .conn
            .prepare("SELECT 1 FROM entries WHERE path = ?1")
            .map_err(ProgramError::RusqliteError)?
            .query_row([path], |_| Ok(()))
            .optional()
            .map_err(ProgramError::RusqliteError)?
            .is_some())
    }

    /// Tags an entry, returns (number of tags created, number of tags added).
    /// Creates a new entry if need.
    pub fn tag_entry(&mut self, entry: &str, tags: &[Tag]) -> Result<(i64, i64), ProgramError> {
        if tags.is_empty() {
            return Ok((0, 0));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        tx.execute("INSERT OR IGNORE INTO entries (path) VALUES (?1)", [entry])
            .map_err(ProgramError::RusqliteError)?;

        let mut created = 0;
        let mut added = 0;
        for tag in tags {
            created += i64::try_from(
                tx.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
                    .map_err(ProgramError::RusqliteError)?,
            )
            .unwrap();
            added += i64::try_from(
                tx.execute(
                    "INSERT OR IGNORE INTO entry_tags (entry, tag) VALUES (?1, ?2)",
                    (entry, tag),
                )
                .map_err(ProgramError::RusqliteError)?,
            )
            .unwrap();
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok((created, added))
    }

    /// Removes tags from an entry, returns the number of tags removed.
    /// Removes the entry if the `tags` is empty.
    pub fn untag_entry(&mut self, entry: &str, tags: &[Tag]) -> Result<i64, ProgramError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        let num_tags: i64 = tx
            .query_one(
                "SELECT count(1) FROM entry_tags WHERE entry = ?1",
                [entry],
                |row| row.get(0),
            )
            .map_err(ProgramError::RusqliteError)?;

        let mut removed = 0;
        for tag in tags {
            removed += i64::try_from(
                tx.execute(
                    "DELETE FROM entry_tags WHERE entry = ?1 AND tag = ?2",
                    (entry, tag),
                )
                .map_err(ProgramError::RusqliteError)?,
            )
            .unwrap();
        }

        if tags.is_empty() || removed == num_tags {
            tx.execute("DELETE FROM entries WHERE path = ?1", [entry])
                .map_err(ProgramError::RusqliteError)?;
            removed = num_tags;
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok(removed)
    }

    /// Returns all entries with `required` tags, excluding those with `forbidden` tags.
    pub fn query(&self, required: &[Tag], forbidden: &[Tag]) -> Result<Vec<String>, ProgramError> {
        let mut sql = "SELECT entry FROM entry_tags GROUP BY entry".to_owned();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();

        let mut conditions = Vec::new();

        if !forbidden.is_empty() {
            conditions.push(format!(
                "SUM(CASE WHEN tag IN ({}) THEN 1 ELSE 0 END) = 0",
                vec!["?"; forbidden.len()].join(",")
            ));
            params.extend(forbidden.iter().map(|t| t as &dyn rusqlite::ToSql));
        }

        if !required.is_empty() {
            conditions.push(format!(
                "COUNT(DISTINCT CASE WHEN tag IN ({}) THEN tag END) = {}",
                vec!["?"; required.len()].join(","),
                required.len()
            ));
            params.extend(required.iter().map(|t| t as &dyn rusqlite::ToSql));
        }

        if !conditions.is_empty() {
            sql.push_str(" HAVING ");
            sql.push_str(&conditions.join(" AND "));
        }

        self.conn
            .prepare(&sql)
            .map_err(ProgramError::RusqliteError)?
            .query_map(params.as_slice(), |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProgramError::RusqliteError)
    }

    /// Returns (number of entries, number of tags)
    pub fn stats(&self) -> Result<(i64, i64), ProgramError> {
        let entries = self
            .conn
            .prepare("SELECT count(1) FROM entries")
            .map_err(ProgramError::RusqliteError)?
            .query_one([], |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?;

        let tags = self
            .conn
            .prepare("SELECT count(1) FROM tags")
            .map_err(ProgramError::RusqliteError)?
            .query_one([], |row| row.get(0))
            .map_err(ProgramError::RusqliteError)?;

        Ok((entries, tags))
    }

    /// Tags multiple files.
    /// Returns (tags created, tags added, files tagged);
    pub fn tag_multiple(
        &mut self,
        entries: impl Iterator<Item = String>,
        tags: &[Tag],
        confirm: Confirm,
    ) -> Result<(i64, i64, i64), ProgramError> {
        if tags.is_empty() {
            return Ok((0, 0, 0));
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        let mut created = 0;
        let mut added = 0;
        let mut files = 0;
        for tag in tags {
            created += i64::try_from(
                tx.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
                    .map_err(ProgramError::RusqliteError)?,
            )
            .unwrap();
        }

        for entry in entries {
            files += i64::try_from(
                tx.execute("INSERT OR IGNORE INTO entries (path) VALUES (?1)", [&entry])
                    .map_err(ProgramError::RusqliteError)?,
            )
            .unwrap();
            for tag in tags {
                added += i64::try_from(
                    tx.execute(
                        "INSERT OR IGNORE INTO entry_tags (entry, tag) VALUES (?1, ?2)",
                        (&entry, tag),
                    )
                    .map_err(ProgramError::RusqliteError)?,
                )
                .unwrap();
            }
        }

        if files == 0 {
            return Ok((0, 0, 0));
        }

        println!("{created} tags will be created");
        println!("{added} tags will be added");
        println!("{files} files will be tagged");

        if !confirm() {
            return Err(ProgramError::UserCanceled);
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok((created, added, files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags() {
        let tag1 = &Tag::new("music").unwrap();
        let tag2 = &Tag::new("photo").unwrap();
        let tag3 = &Tag::new("video").unwrap();

        let mut app = App::new_in_memory().unwrap();
        app.create_tag(tag1).unwrap();
        assert_eq!(app.tags(None).unwrap(), [tag1]);
        app.create_tag(tag2).unwrap();
        assert_eq!(app.tags(None).unwrap(), [tag1, tag2]);
        app.remove_tag(tag1, || true).unwrap();
        assert_eq!(app.tags(None).unwrap(), [tag2]);
        app.rename_tag(tag2, tag1, || true).unwrap();
        assert_eq!(app.tags(None).unwrap(), [tag1]);
        app.rename_tag(tag3, tag1, || true).unwrap_err();
        app.create_tag(tag2).unwrap();
        app.create_tag(tag3).unwrap();
        assert_eq!(app.tags(None).unwrap(), [tag1, tag2, tag3]);
        app.merge_tags(tag1, tag2, Some(tag3), || true).unwrap_err();
        app.merge_tags(tag2, tag3, None, || true).unwrap();
        app.merge_tags(tag1, tag2, Some(tag3), || true).unwrap();
    }
}
