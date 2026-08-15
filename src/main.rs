mod app;
mod cli;
mod errors;
mod finder;
mod path;
mod query_tag;
mod tag;

use std::{path::PathBuf, process::exit};

use clap::Parser;

use crate::{
    app::App,
    cli::*,
    errors::ProgramError,
    finder::{FileScanner, find_db},
    path::resolve_path,
    query_tag::QueryTag,
};

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
        Commands::Tags { file } => {
            let mut tags = db.tags(file.map(resolve_path).transpose()?.as_deref())?;
            tags.sort_unstable();
            for tag in tags {
                println!("{tag}");
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
