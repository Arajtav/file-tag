// TODO: well this works and is better than nothing, however it is terrible.
// By that I mean it only seems to complete subcommands and flags but not tags.
// And well it only really suggests files even when those are obviously not valid.

use clap::CommandFactory;
use clap_complete::{Shell, generate};
use file_tag::cli::Cli;
use std::env;
use std::io;

fn main() -> io::Result<()> {
    let shell: Shell = env::args()
        .nth(1)
        .expect("usage: cargo run --example completions -- <shell>")
        .parse()
        .expect("invalid shell");

    let mut command = Cli::command();

    generate(shell, &mut command, "file-tag", &mut io::stdout());

    Ok(())
}
