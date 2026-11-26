use rusqlite_migration::{M, Migrations};

static MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up(
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
    ),
    // These checks here are just in case I miss some check in rust.
    M::up(
        r"
        CREATE TABLE aliases (
            alias TEXT PRIMARY KEY,
            tag TEXT NOT NULL,
            FOREIGN KEY(tag) REFERENCES tags(name)
                ON DELETE CASCADE
                ON UPDATE CASCADE,
            CHECK (alias <> tag)
        );

        CREATE TRIGGER aliases_not_a_tag_insert
        BEFORE INSERT ON aliases
        WHEN EXISTS (SELECT 1 FROM tags WHERE name = NEW.alias)
        BEGIN
            SELECT RAISE(ABORT, 'NEW_ALIAS_IS_AN_EXISTING_TAG');
        END;

        CREATE TRIGGER aliases_not_a_tag_update
        BEFORE UPDATE OF alias ON aliases
        WHEN EXISTS (SELECT 1 FROM tags WHERE name = NEW.alias)
        BEGIN
            SELECT RAISE(ABORT, 'NEW_ALIAS_IS_AN_EXISTING_TAG');
        END;

        CREATE TRIGGER tags_not_an_alias_insert
        BEFORE INSERT ON tags
        WHEN EXISTS (SELECT 1 FROM aliases WHERE alias = NEW.name)
        BEGIN
            SELECT RAISE(ABORT, 'NEW_TAG_IS_AN_EXISTING_ALIAS');
        END;

        CREATE TRIGGER tags_not_an_alias_update
        BEFORE UPDATE OF name ON tags
        WHEN EXISTS (SELECT 1 FROM aliases WHERE alias = NEW.name)
        BEGIN
            SELECT RAISE(ABORT, 'NEW_TAG_IS_AN_EXISTING_ALIAS');
        END;
        ",
    ),
];
pub const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);
