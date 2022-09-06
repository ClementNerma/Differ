use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::fs::MetadataExt, path::Path};
use walkdir::WalkDir;

// TODO: cross-platform?
// TODO: get actual file size (not physical size)

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    // TODO: add checksum
    // TODO: add creation date
    pub path: String,
    pub items: Vec<SnapshotItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotItem {
    pub path: String,
    pub metadata: SnapshotItemMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SnapshotItemMetadata {
    Directory,
    File(SnapshotComparableFileMetadata),
}

impl SnapshotItemMetadata {
    pub fn size(&self) -> Option<u64> {
        match self {
            Self::Directory => None,
            Self::File(m) => Some(m.size),
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            Self::Directory => true,
            Self::File(_) => false,
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            Self::Directory => false,
            Self::File(_) => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SnapshotComparableFileMetadata {
    pub creation_date: i64,
    pub modif_date: i64,
    pub size: u64,
}

pub fn make_snapshot(base: &Path) -> Result<Snapshot> {
    if !base.is_dir() {
        bail!("Base directory was not found!");
    }

    let path = fs::canonicalize(base)
        .with_context(|| {
            format!(
                "Failed to canonicalize base directory at: {}",
                base.display()
            )
        })?
        .to_str()
        .with_context(|| {
            format!(
                "Base directory contains invalid UTF-8 characters: {}",
                base.display()
            )
        })?
        .to_string();

    let mut items = WalkDir::new(base)
        .min_depth(1)
        .into_iter()
        .par_bridge()
        .map(|item| {
            let item = item?;
            let path = item.path();

            if path.is_symlink() {
                // TODO: symbolic links
                bail!("Warning: ignored symbolic link: {}", path.display())
            } else if path.is_dir() {
                Ok(SnapshotItem {
                    path: get_relative_utf8_path(path, base)?.to_string(),
                    metadata: SnapshotItemMetadata::Directory,
                })
            } else if path.is_file() {
                let metadata = path.metadata().context("Failed to get file's metadata")?;

                // TODO: get real size
                Ok(SnapshotItem {
                    path: get_relative_utf8_path(path, base)?.to_string(),
                    metadata: SnapshotItemMetadata::File(SnapshotComparableFileMetadata {
                        creation_date: metadata.ctime(),
                        modif_date: metadata.mtime(),
                        size: metadata.len(),
                    }),
                })
            } else {
                bail!("Encountered unknown item type at: {}", path.display())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    items.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(Snapshot { path, items })
}

pub fn get_relative_utf8_path<'a>(path: &'a Path, source: &Path) -> Result<&'a str> {
    path.strip_prefix(source)
        .expect("Internal error: failed to strip prefix")
        .to_str()
        .with_context(|| {
            format!(
                "Directory path contains invalid UTF-8 characters: {}",
                path.display()
            )
        })
}
