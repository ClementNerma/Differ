use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::prelude::MetadataExt, path::Path};

// TODO: fail on invalid filenames?
// TODO: cross-platform?

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    // TODO: add checksum
    // TODO: add creation date
    content: SnapshotDir,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SnapshotItem {
    Directory(SnapshotDir),
    File(SnapshotFile),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotDir {
    name: String,
    items: Vec<SnapshotItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotFile {
    name: String,
    creation_date: i64,
    modif_date: i64,
    size: u64,
}

pub fn make_snapshot(source: &Path) -> Result<Snapshot> {
    if !source.is_dir() {
        bail!("Source directory was not found!");
    }

    make_dir_snapshot(source).map(|content| Snapshot { content })
}

fn make_dir_snapshot(source: &Path) -> Result<SnapshotDir> {
    let name = source
        .file_name()
        .with_context(|| {
            format!(
                "Found directory without a name: {}",
                source.to_string_lossy()
            )
        })?
        .to_string_lossy()
        .to_string();

    let mut items = vec![];

    for item in fs::read_dir(source)
        .with_context(|| format!("Failed to read directory: {}", source.to_string_lossy()))?
    {
        let path = item?.path();

        if path.is_symlink() {
            // TODO: symbolic links
            bail!("Warning: ignored symbolic link: {}", path.to_string_lossy());
        } else if path.is_dir() {
            items.push(SnapshotItem::Directory(make_dir_snapshot(&path)?));
        } else if path.is_file() {
            items.push(SnapshotItem::File(make_file_snapshot(&path).with_context(
                || format!("Encountered error at file: {}", path.to_string_lossy()),
            )?));
        } else {
            bail!(
                "Encountered unknown item type at: {}",
                path.to_string_lossy()
            );
        }
    }

    Ok(SnapshotDir { name, items })
}

fn make_file_snapshot(source: &Path) -> Result<SnapshotFile> {
    let metadata = source.metadata().context("Failed to get file's metadata")?;

    Ok(SnapshotFile {
        name: source
            .file_name()
            .map(|str| str.to_string_lossy().to_string())
            .context("File does not have a filename")?,
        size: metadata.size(),
        creation_date: metadata.ctime(),
        modif_date: metadata.mtime(),
    })
}
