mod app;
mod errors;
mod finder;
mod migrations;
mod path;
mod query_tag;
mod tag;

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand, ValueEnum};
use itertools::Either;

use crate::{
    app::App,
    errors::ProgramError,
    finder::{FileScanner, find_db},
    path::resolve_path,
    query_tag::QueryTag,
    tag::Tag,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TagSort {
    /// Sort alphabetically
    Name,
    /// Sort by the number of files using the tag
    Count,
}

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
        /// Print the total number of files using each tag.
        #[arg(short, long)]
        count: bool,
        /// Sort the output.
        #[arg(short, long, default_value = "name")]
        sort: TagSort,
        /// Reverse the sort order.
        #[arg(short, long)]
        reverse: bool,
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

fn confirm() -> bool {
    matches!(inquire::Confirm::new("Are you sure?").prompt(), Ok(true))
}

fn always_confirm() -> bool {
    true
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<(), ProgramError> {
    let args = Cli::parse();

    // that split here is ugly, but otherwise that command could be blocked
    // if it is not possible to create a global database.
    if matches!(args.command, Commands::InitLocal) {
        return App::new(&PathBuf::from("file-tag.sqlite")).map(|_| ());
    }

    let db = find_db()?;

    match args.command {
        Commands::Database => {
            println!("{}", db.display());
            return Ok(());
        }
        Commands::Backup => {
            if std::fs::exists(&db).is_ok_and(|a| a) {
                let time = chrono::Utc::now();

                let new = db.with_file_name(format!(
                    "{}-{}.sqlite",
                    db.file_stem().unwrap().to_string_lossy(),
                    time.to_rfc3339()
                ));

                match std::fs::copy(db, &new) {
                    Ok(_) => println!("Backup of the database created at {}", new.display()),
                    Err(_) => eprintln!("Failed to create a backup"),
                }
            } else {
                println!("No database exists yet to be backed up");
            }

            return Ok(());
        }
        _ => {}
    }

    let mut db = App::new(&db)?;
    match args.command {
        Commands::InitLocal | Commands::Database | Commands::Backup => unreachable!(),
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
        Commands::Tags {
            file,
            count,
            sort,
            reverse,
        } => {
            let file = file.map(resolve_path).transpose()?;

            if count || sort == TagSort::Count {
                let mut rows = db.tags_counted(file.as_deref())?;

                match sort {
                    // The name is first in the tuple.
                    TagSort::Name => rows.sort_unstable(),
                    TagSort::Count => {
                        rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    }
                }

                let rows = if reverse {
                    Either::Left(rows.into_iter().rev())
                } else {
                    Either::Right(rows.into_iter())
                };

                for (tag, n) in rows {
                    if count {
                        println!("{n}\t{tag}");
                    } else {
                        println!("{tag}");
                    }
                }
            } else {
                let mut tags = db.tags(file.as_deref())?;

                tags.sort_unstable();

                let tags = if reverse {
                    Either::Left(tags.into_iter().rev())
                } else {
                    Either::Right(tags.into_iter())
                };

                for tag in tags {
                    println!("{tag}");
                }
            }
        }
        Commands::Tag { file, tags } => {
            // if the file does not exist it will print that too.
            if file.is_symlink() || !file.is_file() {
                eprintln!("only files can be tagged");
                return Ok(());
            }

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
        Commands::DirTag { dir, tags, yes } => {
            let fs = FileScanner::new(dir);
            db.tag_multiple(
                fs.into_iter().filter_map(|f| resolve_path(f).ok()),
                &tags,
                if yes { always_confirm } else { confirm },
            )?;
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
