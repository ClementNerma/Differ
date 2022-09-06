use anyhow::Result;
use std::{
    collections::HashSet, ffi::OsStr, fs::canonicalize, os::unix::prelude::MetadataExt, path::Path,
};

use anyhow::{bail, Context};
use rayon::prelude::{ParallelBridge, ParallelIterator};
use walkdir::WalkDir;

use super::{Driver, DriverFileMetadata, DriverItem, DriverItemMetadata};

pub struct FsDriver;

impl FsDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FsDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for FsDriver {
    fn find_all(&self, root: &str, ignore: &HashSet<&str>) -> Result<Vec<DriverItem>> {
        let ignore: HashSet<_> = ignore.iter().map(OsStr::new).collect();

        let root = canonicalize(root)
            .with_context(|| format!("Failed to canonicalize base directory at: {root}"))?;

        let root = Path::new(root.to_str().with_context(|| {
            format!(
                "Base directory contains invalid UTF-8 characters: {}",
                root.display()
            )
        })?);

        if !root.is_dir() {
            bail!("Root directory was not found!")
        }

        WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| {
                !entry
                    .path()
                    .ancestors()
                    .any(|ancestor| match ancestor.file_name() {
                        Some(name) => ignore.contains(name),
                        None => false,
                    })
            })
            .par_bridge()
            .map(|item| {
                let item = item.context("Failed to access item")?;
                let item = item.path();

                let path = get_relative_utf8_path(item, root)?.to_string();

                if item.is_symlink() {
                    // TODO: symbolic links
                    bail!("Warning: ignored symbolic link: {}", item.display())
                } else if item.is_dir() {
                    Ok(Some(DriverItem {
                        path,
                        metadata: DriverItemMetadata::Directory,
                    }))
                } else if item.is_file() {
                    let metadata = item.metadata().with_context(|| {
                        format!("Failed to get file's metadata for: {}", item.display())
                    })?;

                    // TODO: get real size
                    Ok(Some(DriverItem {
                        path,
                        metadata: DriverItemMetadata::File(DriverFileMetadata {
                            // creation_date: metadata.ctime(),
                            modification_date: metadata.mtime(),
                            size: metadata.len(),
                        }),
                    }))
                } else {
                    bail!("Encountered unknown item type at: {}", item.display())
                }
            })
            .filter_map(|r| r.transpose())
            .collect::<Result<Vec<_>, _>>()
    }
}

fn get_relative_utf8_path<'a>(path: &'a Path, source: &Path) -> Result<&'a str> {
    path.strip_prefix(source)
        .context("Internal error: failed to strip prefix")?
        .to_str()
        .with_context(|| {
            format!(
                "Item path contains invalid UTF-8 characters: {}",
                path.display()
            )
        })
}
