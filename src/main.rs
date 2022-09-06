#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
pub mod diffing;
pub mod drivers;
mod logging;

// use std::time::Instant;

use crate::{
    diffing::{build_diff, CategorizedDiff},
    drivers::{fs::FsDriver, make_snapshot, DriverItemMetadata},
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

    let driver = FsDriver::new();

    info!("Building snapshots for source and destination...");

    // let started = Instant::now();

    let (source, backup) = std::thread::scope(|s| {
        let source = s.spawn(|| {
            make_snapshot(
                &driver,
                cmd.source_dir
                    .to_str()
                    .expect("Source path contains non-UTF-8 characters"),
            )
            .unwrap()
        });

        let backup = s.spawn(|| {
            make_snapshot(
                &driver,
                cmd.backup_dir
                    .to_str()
                    .expect("Backup path contains non-UTF-8 characters"),
            )
            .unwrap()
        });

        (source.join().unwrap(), backup.join().unwrap())
    });

    // println!("Snapshots built in {}s.", started.elapsed().as_secs());

    // let started = Instant::now();

    let mut diff = build_diff(source, backup);
    diff.sort();

    let cat = CategorizedDiff::new(diff);

    if !cat.added.is_empty() {
        println!("Added:");

        for (path, added) in &cat.added {
            match added.new {
                DriverItemMetadata::Directory => {
                    println!(" {}", format!("{}/", path).bright_green())
                }
                DriverItemMetadata::File(m) => println!(
                    " {} {}",
                    path.bright_green(),
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
                "{}",
                format!(" {} ({})", path, human_size(modified.new.size)).bright_yellow()
            );
        }

        println!();
    }

    if !cat.type_changed.is_empty() {
        println!("Type changed:");

        let type_letter = |m: DriverItemMetadata| match m {
            DriverItemMetadata::Directory => "D",
            DriverItemMetadata::File(_) => "F",
        };

        for (path, type_changed) in &cat.type_changed {
            let message = format!(
                " {}{} ({} => {})",
                path,
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
                DriverItemMetadata::Directory => {
                    println!(" {}", format!("{path}/").bright_red())
                }
                DriverItemMetadata::File(m) => println!(
                    " {} {}",
                    path.bright_red(),
                    format!("({})", human_size(m.size)).bright_yellow()
                ),
            }
        }

        println!();
    }

    // println!("Diffing made in {}s.", started.elapsed().as_secs());

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
