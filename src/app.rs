use std::path::Path;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{errors::ProgramError, tag::Tag};

/// App struct.
/// Encapsulates all the backend logic.
pub struct App {
    /// Database connection.
    conn: Connection,
}

/// A function to call to get user confirmation if needed.
pub type Confirm = fn() -> bool;

impl App {
    /// Opens a database connection from `db_path`.
    /// Creates and initializes a new database if needed.
    pub fn new(db_path: &Path) -> Result<Self, ProgramError> {
        let conn = Connection::open(db_path).map_err(ProgramError::RusqliteError)?;

        conn.execute_batch(
            r"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS entries (
                path TEXT PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS entry_tags (
                entry TEXT NOT NULL,
                tag TEXT NOT NULL,
                FOREIGN KEY(entry) REFERENCES entries(path)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY(tag) REFERENCES tags(name)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                PRIMARY KEY(entry, tag)
            );
            ",
        )
        .map_err(ProgramError::RusqliteError)?;
        Ok(App { conn })
    }

    /// Counts how many files is the `tag` on.
    fn count_uses(tx: &Transaction, tag: &Tag) -> Result<usize, ProgramError> {
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

        println!(
            "{} files are tagged with that tag",
            App::count_uses(&tx, tag)?
        );

        if !confirm() {
            return Err(ProgramError::UserCanceled);
        }

        tx.execute("DELETE FROM tags WHERE name = ?1", [tag])
            .map_err(ProgramError::RusqliteError)?;

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
            "UPDATE entry_tags SET tag = ?1 WHERE tag = ?2",
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
    pub fn tags(&self, entry: Option<&str>) -> Result<Vec<String>, ProgramError> {
        Ok(match entry {
            Some(entry) => self
                .conn
                .prepare("SELECT tag FROM entry_tags WHERE entry = ?1")
                .map_err(ProgramError::RusqliteError)?
                .query_map([entry], |row| row.get(0))
                .map_err(ProgramError::RusqliteError)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(ProgramError::RusqliteError)?,
            None => self
                .conn
                .prepare("SELECT name FROM tags")
                .map_err(ProgramError::RusqliteError)?
                .query_map([], |row| row.get(0))
                .map_err(ProgramError::RusqliteError)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(ProgramError::RusqliteError)?,
        })
    }

    /// Tags an entry, returns (number of tags created, number of tags added).
    /// Creates a new entry if need.
    pub fn tag_entry(&mut self, entry: &str, tags: &[Tag]) -> Result<(usize, usize), ProgramError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        tx.execute("INSERT OR IGNORE INTO entries (path) VALUES (?1)", [entry])
            .map_err(ProgramError::RusqliteError)?;

        let mut created = 0;
        let mut added = 0;
        for tag in tags {
            created += tx
                .execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
                .map_err(ProgramError::RusqliteError)?;
            added += tx
                .execute(
                    "INSERT OR IGNORE INTO entry_tags (entry, tag) VALUES (?1, ?2)",
                    (entry, tag),
                )
                .map_err(ProgramError::RusqliteError)?;
        }

        tx.commit().map_err(ProgramError::RusqliteError)?;
        Ok((created, added))
    }

    /// Removes tags from an entry, returns the number of tags removed.
    /// Removes the entry if the `tags` is empty.
    pub fn untag_entry(&mut self, entry: &str, tags: &[Tag]) -> Result<usize, ProgramError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ProgramError::RusqliteError)?;

        let mut removed = 0;
        if tags.is_empty() {
            removed += tx
                .execute("DELETE FROM entry_tags WHERE entry = ?1", [entry])
                .map_err(ProgramError::RusqliteError)?;
            tx.execute("DELETE FROM entries WHERE path = ?1", [entry])
                .map_err(ProgramError::RusqliteError)?;
        } else {
            for tag in tags {
                removed += tx
                    .execute(
                        "DELETE FROM entry_tags WHERE entry = ?1 AND tag = ?2",
                        (entry, tag),
                    )
                    .map_err(ProgramError::RusqliteError)?;
            }
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
}
