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

fn main() {
    let cmd = Args::parse();

    println!("Building source directory snapshot...");
    let source = make_snapshot(&cmd.source_dir).unwrap();

    println!("Building backup directory snapshot...");
    let backup = make_snapshot(Path::new(&cmd.backup_dir)).unwrap();

    println!("Diffing...");
    let diff = build_diff(source, backup);

    for item in diff {
        let sym = match item.status {
            DiffType::Added => "+",
            DiffType::Changed => "~",
            DiffType::TypeChanged => "!",
            DiffType::Deleted => "-",
        };

        println!("{} {}", sym, item.path.display());
    }
}
