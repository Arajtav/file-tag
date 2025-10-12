use inquire::Confirm;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub struct App {
    conn: Connection,
}

pub enum RenameError {
    Canceled,
    TagNotFound,
    TagAlreadyExists,
}

pub enum RemoveError {
    Canceled,
    TagNotFound,
}

pub enum MergeError {
    Canceled,
    TagANotFound,
    TagBNotFound,
    TagAlreadyExists,
    SelfMerge,
}

impl App {
    pub fn new() -> Self {
        let conn = Connection::open("file-tag.sqlite").unwrap();

        conn.execute_batch(
            r#"
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
            "#,
        )
        .unwrap();
        App { conn }
    }

    fn count_uses(tx: &Transaction, tag: &str) -> Option<usize> {
        let tag_exists: bool = tx
            .prepare("SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)")
            .unwrap()
            .query_row([tag], |row| row.get(0))
            .unwrap();

        if !tag_exists {
            return None;
        }

        let count = tx
            .prepare("SELECT COUNT(*) FROM entry_tags WHERE tag = ?1")
            .unwrap()
            .query_row([tag], |row| row.get(0))
            .unwrap();

        Some(count)
    }

    /// Creates a new tag.
    /// Returns false if the tag already exists.
    pub fn create_tag(&mut self, tag: &str) -> bool {
        self.conn
            .execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
            .unwrap()
            != 0
    }

    /// Removes an existing tag.
    pub fn remove_tag(&mut self, tag: &str) -> Result<(), RemoveError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();

        match App::count_uses(&tx, tag) {
            Some(uses) => println!("{uses} files are tagged with that tag"),
            None => {
                return Err(RemoveError::TagNotFound);
            }
        };

        if matches!(Confirm::new("Are you sure?").prompt(), Ok(false) | Err(_)) {
            return Err(RemoveError::Canceled);
        }

        tx.execute("DELETE FROM tags WHERE name = ?1", [tag])
            .unwrap();

        tx.commit().unwrap();
        Ok(())
    }

    /// Renames a tag.
    pub fn rename_tag(&mut self, old: &str, new: &str) -> Result<(), RenameError> {
        if old == new {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();

        if !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [old],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        {
            return Err(RenameError::TagNotFound);
        }

        if tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [new],
                |row| row.get(0),
            )
            .unwrap()
        {
            return Err(RenameError::TagAlreadyExists);
        }

        if matches!(Confirm::new("Are you sure?").prompt(), Ok(false) | Err(_)) {
            return Err(RenameError::Canceled);
        }

        tx.execute("UPDATE tags SET name = ?1 WHERE name = ?2", [new, old])
            .unwrap();

        tx.commit().unwrap();
        Ok(())
    }

    /// Merges 2 tags.
    /// If `new` is `None`, the result is saved under the name of tag_a.
    pub fn merge_tags(
        &mut self,
        tag_a: &str,
        tag_b: &str,
        new: &Option<String>,
    ) -> Result<(), MergeError> {
        if tag_a == tag_b {
            return Err(MergeError::SelfMerge);
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();

        println!(
            "{} files are tagged with {tag_a:?}",
            App::count_uses(&tx, tag_a).ok_or(MergeError::TagANotFound)?
        );

        println!(
            "{} files are tagged with {tag_b:?}",
            App::count_uses(&tx, tag_b).ok_or(MergeError::TagBNotFound)?
        );

        if let Some(new) = new {
            if new != tag_a && new != tag_b {
                if tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                        [new],
                        |row| row.get(0),
                    )
                    .unwrap()
                {
                    return Err(MergeError::TagAlreadyExists);
                }
            }
        }

        if matches!(Confirm::new("Are you sure?").prompt(), Ok(false) | Err(_)) {
            return Err(MergeError::Canceled);
        }

        tx.execute(
            "UPDATE entry_tags SET tag = ?1 WHERE tag = ?2",
            [tag_a, tag_b],
        )
        .unwrap();

        tx.execute("DELETE FROM tags WHERE name = ?1", [tag_b])
            .unwrap();

        if let Some(new) = new.as_deref() {
            tx.execute("UPDATE tags SET name = ?1 WHERE name = ?2", [new, tag_a])
                .unwrap();
        }

        tx.commit().unwrap();
        Ok(())
    }

    /// Returns the list of tags.
    pub fn tags(&self, entry: Option<&str>) -> Vec<String> {
        match entry {
            Some(entry) => self
                .conn
                .prepare("SELECT tag FROM entry_tags WHERE entry = ?1")
                .unwrap()
                .query_map([entry], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            None => self
                .conn
                .prepare("SELECT name FROM tags")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
        }
    }

    // Tags an entry, returns (number of tags created, number of tags added).
    pub fn tag_entry(&mut self, entry: &str, tags: &[String]) -> (usize, usize) {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();

        tx.execute("INSERT OR IGNORE INTO entries (path) VALUES (?1)", [entry])
            .unwrap();

        let mut created = 0;
        let mut added = 0;
        for tag in tags {
            created += tx
                .execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])
                .unwrap();
            added += tx
                .execute(
                    "INSERT OR IGNORE INTO entry_tags (entry, tag) VALUES (?1, ?2)",
                    [entry, tag],
                )
                .unwrap();
        }

        tx.commit().unwrap();
        (created, added)
    }

    // Removes tags from an entry (or removes the entry if `tags` is empty).
    // Returns the number of tags removed.
    pub fn untag_entry(&mut self, entry: &str, tags: &[String]) -> usize {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();

        let mut removed = 0;
        if tags.is_empty() {
            removed += tx
                .execute("DELETE FROM entry_tags WHERE entry = ?1", [entry])
                .unwrap();
        } else {
            for tag in tags {
                removed += tx
                    .execute(
                        "DELETE FROM entry_tags WHERE entry = ?1 AND tag = ?2",
                        [entry, tag],
                    )
                    .unwrap();
            }
        }

        tx.commit().unwrap();
        removed
    }
}
