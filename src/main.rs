#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
pub mod diffing;
mod logging;

use crate::diffing::{build_diff, make_snapshot, DiffType};
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

    println!();

    for item in diff.items() {
        let symbol = match item.status {
            DiffType::Added { new: _ } => "+",
            DiffType::Modified { prev: _, new: _ } => "~",
            DiffType::TypeChanged { prev: _, new: _ } => "!",
            DiffType::Deleted { prev: _ } => "-",
        };

        let message = format!(
            "{} {}{} {}",
            symbol,
            item.path.display(),
            if matches!(item.status.get_new_metadata(), Some(m) if m.is_dir()) {
                "/"
            } else {
                ""
            },
            item.status
                .get_new_metadata()
                .and_then(|m| m.size())
                .map(|s| format!("({})", human_size(s)))
                .unwrap_or(String::new())
        );

        let message = match item.status {
            DiffType::Added { new: _ } => message.bright_green(),
            DiffType::Modified { prev: _, new: _ } => message.bright_yellow(),
            DiffType::TypeChanged { prev: _, new: _ } => message.bright_yellow(),
            DiffType::Deleted { prev: _ } => message.bright_red(),
        };

        println!("{}", message);
    }
}
