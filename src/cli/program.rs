use std::collections::HashSet;
use std::io::{stdout, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::cmd::Args;
use crate::drivers::OnItemHandler;
use crate::drivers::{sftp::SftpDriver, Driver};
use crate::info;
use crate::{
    diffing::{build_diff, CategorizedDiff},
    drivers::{fs::FsDriver, make_snapshot, DriverItemMetadata},
};
use anyhow::{anyhow, bail, Context, Error, Result};
use clap::StructOpt;
use colored::Colorize;

pub fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{}", format!("{:?}", err).bright_red());
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

fn driver_from_arg(arg: &str) -> Result<(Box<dyn Driver + Send + Sync>, String)> {
    if let Some(arg) = arg.strip_prefix("sftp:") {
        let mut parts = arg.split('|');
        let mut split = parts
            .next()
            .context("Please provide a username for SFTP driver")?
            .split('@');

        let username = split
            .next()
            .context("Please provide a username for SFTP driver")?;
        let address = split
            .next()
            .context("Please provide an address for SFTP driver")?;

        if split.next().is_some() {
            bail!("Only one '@' is allowed in argument for SFTP driver");
        }

        let pub_key_path = parts
            .next()
            .context("Please provide the path to the SSH public key file")?;

        let priv_key_path = parts
            .next()
            .context("Please provide the path to the SSH private key file")?;

        let path = parts
            .next()
            .context("Please provide a directory after SSH key files")?
            .to_string();

        if parts.next().is_some() {
            bail!("Too many separators provided for SFTP driver");
        }

        return Ok((
            Box::new(SftpDriver::connect(
                address,
                username,
                Path::new(pub_key_path),
                Path::new(priv_key_path),
            )?),
            path,
        ));
    }

    Ok((Box::new(FsDriver::new()), arg.to_string()))
}

fn inner_main() -> Result<()> {
    let cmd = Args::parse();

    let (source_driver, source_dir) = driver_from_arg(&cmd.source_dir)?;
    let (dest_driver, dest_dir) = driver_from_arg(&cmd.dest_dir)?;

    let ignore = cmd
        .ignore
        .iter()
        .map(|s| s.as_str())
        .collect::<HashSet<_>>();

    info!("Building snapshots for source and destination...");

    let started = Instant::now();

    let stop_request = Arc::new(AtomicBool::new(false));

    let (source, dest) = std::thread::scope(|s| {
        let (source_update, dest_update) = items_spinner();

        let source = s.spawn(|| {
            make_snapshot(
                source_driver.as_ref(),
                source_dir,
                &ignore,
                Arc::clone(&stop_request),
                Some(source_update),
            )
        });

        let dest = s.spawn(|| {
            make_snapshot(
                dest_driver.as_ref(),
                dest_dir,
                &ignore,
                Arc::clone(&stop_request),
                Some(dest_update),
            )
        });

        let err = |err: Error| -> String {
            format!("{:?}", err)
                .split('\n')
                .map(|line| format!("    {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let (source, dest) = (source.join().unwrap(), dest.join().unwrap());

        if source.is_ok() && dest.is_ok() {
            print!("\r");
        } else {
            println!();
        }

        match (source, dest) {
            (Err(source), Err(dest)) => Err(anyhow!(
                "Source snapshot failed:\n{}\n\nDestination snapshot failed:\n{}",
                err(source).bright_yellow(),
                err(dest).bright_yellow()
            )),

            (Err(source), Ok(_)) => Err(anyhow!(
                "Source snapshot failed:\n{}",
                err(source).bright_yellow()
            )),

            (Ok(_), Err(dest)) => Err(anyhow!(
                "Destination snapshot failed:\n{}",
                err(dest).bright_yellow()
            )),

            (Ok(source), Ok(dest)) => Ok((source, dest)),
        }
    })?;

    info!(
        "Found {} files in source and {} in destination in {}. Computing differences...",
        source.items.len().to_string().bright_yellow(),
        dest.items.len().to_string().bright_yellow(),
        format!("{}s", started.elapsed().as_secs()).bright_magenta()
    );

    let started = Instant::now();

    let mut diff = build_diff(source, dest);
    diff.sort();

    let cat = CategorizedDiff::new(diff);

    if !cat.added.is_empty() {
        info!("Added:");

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
        info!("Modified:");

        for (path, modified) in &cat.modified {
            println!(
                "{}",
                format!(" {} ({})", path, human_size(modified.new.size)).bright_yellow()
            );
        }

        println!();
    }

    if !cat.type_changed.is_empty() {
        info!("Type changed:");

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
        info!("Deleted:");

        for (path, deleted) in &cat.deleted {
            match deleted.prev {
                DriverItemMetadata::Directory => {
                    info!(" {}", format!("{path}/").bright_red())
                }
                DriverItemMetadata::File(m) => info!(
                    " {} {}",
                    path.bright_red(),
                    format!("({})", human_size(m.size)).bright_yellow()
                ),
            }
        }

        info!("");
    }

    info!(
        "Differences computed in {}.",
        format!("{}s", started.elapsed().as_secs()).bright_magenta()
    );

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

    info!(
        "Found a total of {} items to transfer and {} to delete for a total of {}.",
        transfer_count.to_string().bright_green(),
        delete_count.to_string().bright_red(),
        human_size(transfer_size).bright_yellow()
    );

    Ok(())
}

pub fn items_spinner() -> (OnItemHandler, OnItemHandler) {
    let started = Instant::now();

    let started_1 = Arc::new(started);
    let started_2 = Arc::clone(&started_1);

    let src_counter_1 = Arc::new(AtomicU64::new(0));
    let src_counter_2 = Arc::clone(&src_counter_1);

    let dest_counter_1 = Arc::new(AtomicU64::new(0));
    let dest_counter_2 = Arc::clone(&dest_counter_1);

    fn update(src: u64, dest: u64, started: Instant) {
        print!(
            "\rSource: found {src} items | Destination: found {dest} items | Searching for {}s...",
            started.elapsed().as_secs()
        );

        stdout().flush().unwrap();
    }

    update(0, 0, started);

    (
        Box::new(move |_| {
            let updated = src_counter_1.load(Ordering::Acquire) + 1;
            src_counter_1.store(updated, Ordering::Release);

            if updated % 100 == 0 {
                update(updated, dest_counter_1.load(Ordering::Acquire), *started_1);
            }
        }),
        Box::new(move |_| {
            let updated = dest_counter_2.load(Ordering::Acquire) + 1;
            dest_counter_2.store(updated, Ordering::Release);

            if updated % 100 == 0 {
                update(src_counter_2.load(Ordering::Acquire), updated, *started_2);
            }
        }),
    )
}
