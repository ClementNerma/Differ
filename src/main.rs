#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
pub mod diffing;
mod logging;

use crate::diffing::{build_diff, make_snapshot, CategorizedDiff, SnapshotItemMetadata};
use clap::StructOpt;
use cmd::Args;
use colored::Colorize;
use std::path::Path;

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
    let mut diff = build_diff(source, backup);
    diff.sort();

    let cat = CategorizedDiff::new(diff);

    if !cat.added.is_empty() {
        println!("Added:");

        for (path, added) in &cat.added {
            match added.new {
                SnapshotItemMetadata::Directory => {
                    println!(" {}", format!("{}/", path.to_string_lossy()).bright_green())
                }
                SnapshotItemMetadata::File(m) => println!(
                    " {} {}",
                    path.to_string_lossy().bright_green(),
                    format!("({})", human_size(m.size)).bright_yellow()
                ),
            }
        }

        println!();
    }

    if !cat.modified.is_empty() {
        println!("Modified:");

        for (path, modified) in &cat.modified {
            println!(
                " {} {}",
                path.to_string_lossy().bright_yellow(),
                format!("({})", human_size(modified.new.size)).bright_yellow()
            );
        }

        println!();
    }

    if !cat.type_changed.is_empty() {
        println!("Type changed:");

        let type_letter = |m: SnapshotItemMetadata| match m {
            SnapshotItemMetadata::Directory => "D",
            SnapshotItemMetadata::File(_) => "F",
        };

        for (path, type_changed) in &cat.type_changed {
            let message = format!(
                " {}{} ({} => {})",
                path.to_string_lossy(),
                if type_changed.new.is_dir() { "/" } else { "" },
                type_letter(type_changed.prev),
                type_letter(type_changed.new)
            );

            println!("{}", message.bright_yellow());
        }

        println!();
    }

    if !cat.deleted.is_empty() {
        println!("Deleted:");

        for (path, deleted) in &cat.deleted {
            match deleted.prev {
                SnapshotItemMetadata::Directory => {
                    println!(" {}", format!("{}/", path.to_string_lossy()).bright_red())
                }
                SnapshotItemMetadata::File(m) => println!(
                    " {} {}",
                    path.to_string_lossy().bright_red(),
                    format!("({})", human_size(m.size)).bright_yellow()
                ),
            }
        }

        println!();
    }

    let transfer_count = cat.added.len() + cat.modified.len() + cat.type_changed.len();
    let delete_count = cat.type_changed.len() + cat.deleted.len();
    let transfer_size = cat
        .added
        .iter()
        .fold(0, |acc, (_, i)| acc + i.new.size().unwrap_or(0))
        + cat.modified.iter().fold(0, |acc, (_, i)| acc + i.new.size)
        + cat
            .type_changed
            .iter()
            .fold(0, |acc, (_, i)| acc + i.new.size().unwrap_or(0));

    println!(
        "Found a total of {} items to transfer and {} to delete for a total of {}.",
        transfer_count.to_string().bright_green(),
        delete_count.to_string().bright_red(),
        human_size(transfer_size).bright_yellow()
    );
}
