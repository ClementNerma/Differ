#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
mod diff;
mod logging;
mod snapshot;

use std::path::Path;

use crate::{
    diff::{build_diff, DiffType},
    snapshot::{make_snapshot, SnapshotItemMetadata},
};
use clap::StructOpt;
use cmd::Args;
use colored::Colorize;

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{} B", bytes);
    }

    let mut bytes = bytes as f64 / 1024.0;

    if bytes < 1024.0 {
        return format!("{:.2} KiB", bytes);
    }

    bytes /= 1024.0;

    if bytes < 1024.0 {
        return format!("{:.2} MiB", bytes);
    }

    format!("{:.2} GiB", bytes / 1024.0)
}

fn main() {
    let cmd = Args::parse();

    info!("Building source directory snapshot...");
    let source = make_snapshot(&cmd.source_dir).unwrap();

    info!("Building backup directory snapshot...");
    let backup = make_snapshot(Path::new(&cmd.backup_dir)).unwrap();

    info!("Diffing...");
    let diff = build_diff(source, backup);

    println!();

    let item_size =
        |item: SnapshotItemMetadata| item.size().map(human_size).unwrap_or(String::from("-"));

    for item in diff.items() {
        let symbol = match item.status {
            DiffType::Added { new: _ } => "+",
            DiffType::Changed { prev: _, new: _ } => "~",
            DiffType::TypeChanged { prev: _, new: _ } => "!",
            DiffType::Deleted { prev: _ } => "-",
        };

        let size_update = match item.status {
            DiffType::Added { new } => item_size(new),
            DiffType::Changed { prev, new } => format!("{} => {}", item_size(prev), item_size(new)),
            DiffType::TypeChanged { prev, new } => {
                format!("{} => {}", item_size(prev), item_size(new))
            }
            DiffType::Deleted { prev } => item_size(prev),
        };

        let message = format!(
            "{} {} {}",
            symbol,
            item.path.display(),
            format!("({})", size_update) //.bright_yellow()
        );

        let message = match item.status {
            DiffType::Added { new: _ } => message.bright_green(),
            DiffType::Changed { prev: _, new: _ } => message.bright_yellow(),
            DiffType::TypeChanged { prev: _, new: _ } => message.bright_yellow(),
            DiffType::Deleted { prev: _ } => message.bright_red(),
        };

        println!("{}", message);
    }
}
