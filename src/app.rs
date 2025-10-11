use inquire::Confirm;
use rusqlite::{Connection, TransactionBehavior};

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
            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY
            );
            "#,
        )
        .unwrap();
        App { conn }
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
        if !self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [tag],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        {
            return Err(RemoveError::TagNotFound);
        }

        if matches!(Confirm::new("Are you sure?").prompt(), Ok(false) | Err(_)) {
            return Err(RemoveError::Canceled);
        }

        self.conn
            .execute("DELETE FROM tags WHERE name = ?1", [tag])
            .unwrap();

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

        if !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [tag_a],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        {
            return Err(MergeError::TagANotFound);
        }

        if !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1)",
                [tag_b],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        {
            return Err(MergeError::TagBNotFound);
        }

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

        // actual merge here when there when there will be other tables.
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
    pub fn tags(&self) -> Vec<String> {
        self.conn
            .prepare("SELECT name FROM tags")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}
