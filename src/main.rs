#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
mod diff;
mod snapshot;

use std::path::Path;

use crate::{
    diff::{build_diff, DiffType},
    snapshot::make_snapshot,
};
use clap::StructOpt;
use cmd::Args;
use colored::Colorize;

fn main() {
    let cmd = Args::parse();

    println!("{}", "Building source directory snapshot...".blue());
    let source = make_snapshot(&cmd.source_dir).unwrap();

    println!("{}", "Building backup directory snapshot...".blue());
    let backup = make_snapshot(Path::new(&cmd.backup_dir)).unwrap();

    println!("{}", "Diffing...".blue());
    let diff = build_diff(source, backup);

    println!();

    for item in diff.items() {
        let sym = match item.status {
            DiffType::Added => "+",
            DiffType::Changed => "~",
            DiffType::TypeChanged => "!",
            DiffType::Deleted => "-",
        };

        let message = format!("{} {}", sym, item.path.display());

        let message = match item.status {
            DiffType::Added => message.bright_green(),
            DiffType::Changed => message.bright_yellow(),
            DiffType::TypeChanged => message.bright_yellow(),
            DiffType::Deleted => message.bright_red(),
        };

        println!("{}", message);
    }
}
