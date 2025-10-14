mod app;
mod errors;
mod finder;
mod path;
mod query_tag;
mod tag;

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};

use crate::{
    app::App, errors::ProgramError, finder::find_db, path::resolve_path, query_tag::QueryTag,
    tag::Tag,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates a new tag.
    Create { tag: Tag },
    /// Removes a tag.
    Remove { tag: Tag },
    /// Changes the name of a tag.
    Rename { old_tag: Tag, new_tag: Tag },
    /// Merges 2 tags, optionally saving the result under a new name.
    Merge {
        tag_a: Tag,
        tag_b: Tag,
        new_tag: Option<Tag>,
    },
    /// Lists all the tags.
    Tags { file: Option<PathBuf> },
    /// Tags a file.
    Tag {
        file: PathBuf,
        /// Tags to add, if no tags are provided the file entry is added anyways.
        tags: Vec<Tag>,
    },
    /// Removes tags from a file.
    Untag {
        file: PathBuf,
        /// Tags to remove, if no tags are provided the file entry is removed.
        tags: Vec<Tag>,
    },
    /// Queries the database.
    Query { query_tags: Vec<QueryTag> },
    /// Creates a new local database.
    InitLocal,
}

fn run() -> Result<(), ProgramError> {
    let args = Cli::parse();

    // that split here is ugly, but otherwise that command could be blocked
    // if it is not possible to create a global database.
    if matches!(args.command, Commands::InitLocal) {
        return App::new(&PathBuf::from("file-tag.sqlite")).map(|_| ());
    }

    let mut db = App::new(&find_db()?)?;
    match args.command {
        Commands::InitLocal => unreachable!(),
        Commands::Create { tag } => {
            db.create_tag(&tag)?;
            println!("Created tag {tag}");
        }
        Commands::Remove { tag } => {
            db.remove_tag(&tag)?;
            println!("Removed tag {tag}");
        }
        Commands::Rename { old_tag, new_tag } => {
            db.rename_tag(&old_tag, &new_tag)?;
            println!("Renamed {old_tag} to {new_tag}");
        }
        Commands::Merge {
            tag_a,
            tag_b,
            new_tag,
        } => {
            db.merge_tags(&tag_a, &tag_b, new_tag.as_ref())?;
            println!(
                "merged {tag_a} with {tag_b} (saved under {})",
                new_tag.as_ref().unwrap_or(&tag_a)
            );
        }
        Commands::Tags { file } => {
            let mut tags = db.tags(file.map(resolve_path).transpose()?.as_deref())?;
            tags.sort_unstable();
            for tag in tags {
                println!("{tag}")
            }
        }
        Commands::Tag { file, tags } => {
            let (created, added) = db.tag_entry(&resolve_path(file)?, &tags)?;
            println!("{created} tags created, {added} tags added");
        }
        Commands::Untag { file, tags } => {
            let removed = db.untag_entry(&resolve_path(file)?, &tags)?;
            println!("{removed} tags removed");
        }
        Commands::Query { query_tags } => {
            let mut required = Vec::new();
            let mut forbidden = Vec::new();
            for tag in query_tags {
                match tag {
                    QueryTag::Required(tag) => required.push(tag),
                    QueryTag::Forbidden(tag) => forbidden.push(tag),
                }
            }

            let mut results = db.query(&required, &forbidden)?;
            results.sort_unstable();
            for tag in results {
                println!("{tag}")
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        if let ProgramError::UserCanceled = err {
        } else {
            eprintln!("{err}");
            exit(1)
        }
    }
}
