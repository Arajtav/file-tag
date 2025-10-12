mod app;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::app::{App, MergeError, RemoveError, RenameError};

fn validate_tag_name(tag: &str) -> bool {
    !tag.is_empty()
        && !tag.starts_with("_")
        && !tag.ends_with("_")
        && !tag.contains("__")
        && tag.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

macro_rules! verify_tag_name {
    ($tag:expr) => {{
        if !validate_tag_name($tag) {
            eprintln!("{:?} is not a valid tag name", $tag);
            return;
        }
    }};
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates a new tag.
    Create { tag: String },
    /// Removes a tag.
    Remove { tag: String },
    /// Changes the name of a tag.
    Rename { old_tag: String, new_tag: String },
    /// Merges 2 tags, optionally saving the result under a new name.
    Merge {
        tag_a: String,
        tag_b: String,
        new_tag: Option<String>,
    },
    /// Lists all the tags.
    Tags { file: Option<PathBuf> },
    /// Tags a file.
    Tag {
        file: PathBuf,
        /// Tags to add, if no tags are provided the file entry is added anyways.
        tags: Vec<String>,
    },
    /// Removes tags from a file.
    Untag {
        file: PathBuf,
        /// Tags to remove, if no tags are provided the file entry is removed.
        tags: Vec<String>,
    },
}

fn main() {
    let args = Cli::parse();
    let mut db = App::new();
    match args.command {
        Commands::Create { tag } => {
            verify_tag_name!(&tag);
            if db.create_tag(&tag) {
                println!("Successfully created {tag:?}")
            } else {
                eprintln!("{tag:?} already exists");
            }
        }
        Commands::Remove { tag } => match db.remove_tag(&tag) {
            Ok(_) => {
                println!("Removed {tag:?}")
            }
            Err(RemoveError::TagNotFound) => {
                eprintln!("{tag:?} doesn't exist");
            }
            Err(RemoveError::Canceled) => {}
        },
        Commands::Rename { old_tag, new_tag } => {
            verify_tag_name!(&new_tag);
            match db.rename_tag(&old_tag, &new_tag) {
                Ok(_) | Err(RenameError::Canceled) => {}
                Err(RenameError::TagNotFound) => {
                    eprintln!("{old_tag:?} doesn't exist");
                }
                Err(RenameError::TagAlreadyExists) => {
                    eprintln!(
                        "{new_tag:?} already exists, if you want to merge them use the `merge` command"
                    );
                }
            }
        }
        Commands::Merge {
            tag_a,
            tag_b,
            new_tag,
        } => {
            if let Some(new_tag) = &new_tag {
                verify_tag_name!(new_tag);
            }
            match db.merge_tags(&tag_a, &tag_b, &new_tag) {
                Ok(_) | Err(MergeError::Canceled) => {}
                Err(MergeError::TagANotFound) => {
                    eprintln!("{tag_a:?} doesn't exist");
                }
                Err(MergeError::TagBNotFound) => {
                    eprintln!("{tag_b:?} doesn't exist");
                }
                Err(MergeError::TagAlreadyExists) => {
                    eprintln!(
                        "{:?} already exists, if you want to merge them use the `merge` command",
                        new_tag.unwrap()
                    );
                }
                Err(MergeError::SelfMerge) => {
                    eprintln!("Cannot merge {tag_a:?} with itself")
                }
            }
        }
        Commands::Tags { file } => {
            let mut tags = db.tags(
                file.as_ref()
                    .map(|p| p.canonicalize().unwrap())
                    .as_ref()
                    .map(|p| p.to_str().unwrap()),
            );
            tags.sort_unstable();
            for tag in tags {
                println!("{tag:?}")
            }
        }
        Commands::Tag { file, tags } => {
            for tag in &tags {
                verify_tag_name!(tag);
            }

            let (created, added) =
                db.tag_entry(file.canonicalize().unwrap().to_str().unwrap(), &tags);
            println!("{created} tags created, {added} tags added");
        }
        Commands::Untag { file, tags } => {
            let removed = db.untag_entry(file.canonicalize().unwrap().to_str().unwrap(), &tags);
            println!("{removed} tags removed");
        }
    }
}
