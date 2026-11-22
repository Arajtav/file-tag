use rusqlite_migration::{M, Migrations};

static MIGRATIONS_SLICE: &[M<'_>] = &[M::up(
    r"
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
)];
pub const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);
