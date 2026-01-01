mod app;
mod errors;
mod finder;
mod path;
mod query_tag;
mod tag;

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};

use crate::{
    app::App,
    errors::ProgramError,
    finder::{FileScanner, find_db},
    path::resolve_path,
    query_tag::QueryTag,
    tag::Tag,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Lists files from the current directory that are not in the database. Symlinks are ignored.
    Untagged,
    /// Prints the database stats, like the number of files, tags, etc.
    Stats,
}

fn confirm() -> bool {
    matches!(inquire::Confirm::new("Are you sure?").prompt(), Ok(true))
}

fn always_confirm() -> bool {
    true
}

fn run() -> Result<(), ProgramError> {
    let args = Cli::parse();

    // that split here is ugly, but otherwise that command could be blocked
    // if it is not possible to create a global database.
    if matches!(args.command, Commands::InitLocal) {
        return App::new(&PathBuf::from("file-tag.sqlite")).map(|_| ());
    }

    let db = find_db()?;

    if matches!(args.command, Commands::Database) {
        println!("{}", db.display());
        return Ok(());
    }

    let mut db = App::new(&db)?;
    match args.command {
        Commands::InitLocal | Commands::Database => unreachable!(),
        Commands::Create { tag } => {
            db.create_tag(&tag)?;
            println!("Created tag {tag}");
        }
        Commands::Remove { tag, yes } => {
            db.remove_tag(&tag, if yes { always_confirm } else { confirm })?;
            println!("Removed tag {tag}");
        }
        Commands::Rename {
            old_tag,
            new_tag,
            yes,
        } => {
            db.rename_tag(
                &old_tag,
                &new_tag,
                if yes { always_confirm } else { confirm },
            )?;
            println!("Renamed {old_tag} to {new_tag}");
        }
        Commands::Merge {
            tag_a,
            tag_b,
            new_tag,
            yes,
        } => {
            db.merge_tags(
                &tag_a,
                &tag_b,
                new_tag.as_ref(),
                if yes { always_confirm } else { confirm },
            )?;
            println!(
                "merged {tag_a} with {tag_b} (saved under {})",
                new_tag.as_ref().unwrap_or(&tag_a)
            );
        }
        Commands::Tags { file } => {
            let mut tags = db.tags(file.map(resolve_path).transpose()?.as_deref())?;
            tags.sort_unstable();
            for tag in tags {
                println!("{tag}");
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
                println!("{tag}");
            }
        }
        Commands::Untagged => {
            let fs = FileScanner::new(std::env::current_dir().expect("Failed to get cwd"));
            for file in fs {
                let file =
                    resolve_path(file.clone()).unwrap_or(file.to_string_lossy().into_owned());

                if !db.is_in_db(&file)? {
                    println!("{file}");
                }
            }
        }
        Commands::Stats => {
            let (entries, tags) = db.stats()?;
            println!("number of entries: {entries}");
            println!("number of tags: {tags}");
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
