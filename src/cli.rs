use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{query_tag::QueryTag, tag::Tag};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Creates a new tag, or makes sure a tag exists.
    Create {
        /// Tag to create.
        tag: Tag,
    },
    /// Removes a tag and all references to it.
    Remove {
        /// Tag to remove.
        tag: Tag,

        /// Skip confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Changes the name of a tag.
    Rename {
        /// Tag to rename.
        old_tag: Tag,
        /// New name.
        new_tag: Tag,

        /// Skip confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Merges 2 tags.
    Merge {
        /// First tag.
        tag_a: Tag,
        /// Second tag.
        tag_b: Tag,
        /// Optionally a name under which the merged tag should
        /// be saved (uses the name of the first tag otherwise).
        new_tag: Option<Tag>,

        /// Skip confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Lists all the tags.
    Tags {
        /// Optionally file from which the tags will be listed.
        file: Option<PathBuf>,
    },
    /// Tags a file.
    Tag {
        /// File to tag.
        file: PathBuf,
        /// Tags to add.
        tags: Vec<Tag>,
    },
    /// Removes tags from a file.
    Untag {
        /// File to untag.
        file: PathBuf,
        /// Tags to remove, if no tags are provided, all tags are removed,
        /// and the file is removed from tracking.
        tags: Vec<Tag>,
    },
    /// Queries the database.
    Query {
        /// Query tags.
        query_tags: Vec<QueryTag>,
    },
    /// Creates a new local database. Does nothing if the database exists already.
    InitLocal,
    /// Prints the location of the database that will be used.
    Database,
    /// Creates a copy of the database.
    Backup,
    /// Lists files from the current directory that are not in the database. Symlinks are ignored.
    Untagged,
    /// Prints the database stats, like the number of files, tags, etc.
    Stats,
    /// Tags every file in a directory, works recursively.
    DirTag {
        /// Directory to scan.
        dir: PathBuf,

        /// Tags to add.
        tags: Vec<Tag>,

        /// Skip confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
}
