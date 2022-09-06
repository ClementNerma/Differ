use super::cmd::Args;
use crate::drivers::{sftp::SftpDriver, Driver};
use crate::info;
use crate::{
    diffing::{build_diff, CategorizedDiff},
    drivers::{fs::FsDriver, make_snapshot, DriverItemMetadata},
};
use anyhow::{bail, Context, Result};
use clap::StructOpt;
use colored::Colorize;

pub fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{}", err.to_string().bright_red());
        std::process::exit(1);
    }
}

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

fn driver_from_arg(arg: &str) -> Result<Box<dyn Driver + Send + Sync>> {
    if let Some(arg) = arg.strip_prefix("sftp:") {
        let mut split = arg.split('@');

        let username = split
            .next()
            .context("Please provide a username for SFTP driver (<username>@<address>)")?;
        let address = split
            .next()
            .context("Please provide an address for SFTP driver (<username>@<address>)")?;

        if split.next().is_some() {
            bail!("Only one '@' is allowed in argument for SFTP driver");
        }

        return Ok(Box::new(SftpDriver::connect(address, username)?));
    }

    Ok(Box::new(FsDriver::new()))
}

fn inner_main() -> Result<()> {
    let cmd = Args::parse();

    let source_driver = driver_from_arg(&cmd.source_dir).unwrap();
    let dest_driver = driver_from_arg(&cmd.dest_dir).unwrap();

    info!("Building snapshots for source and destination...");

    // let started = Instant::now();

    let (source, dest) = std::thread::scope(|s| {
        let source = s.spawn(|| make_snapshot(source_driver.as_ref(), &cmd.source_dir).unwrap());
        let dest = s.spawn(|| make_snapshot(dest_driver.as_ref(), &cmd.dest_dir).unwrap());

        (source.join().unwrap(), dest.join().unwrap())
    });

    // println!("Snapshots built in {}s.", started.elapsed().as_secs());

    // let started = Instant::now();

    let mut diff = build_diff(source, dest);
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

    Ok(())
}
