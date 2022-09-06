use anyhow::Result;
use std::{fs::canonicalize, os::unix::prelude::MetadataExt, path::Path};

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

impl Driver for FsDriver {
    fn id(&self) -> String {
        "fs:local".to_string()
    }

    fn canonicalize(&self, path: &str) -> Result<String> {
        Ok(canonicalize(path)
            .with_context(|| format!("Failed to canonicalize base directory at: {path}"))?
            .to_str()
            .with_context(|| format!("Base directory contains invalid UTF-8 characters: {path}"))?
            .to_string())
    }

    fn find_all(&self, root: &str) -> Result<Vec<DriverItem>> {
        let root = Path::new(root);

        if !root.is_dir() {
            bail!("Root directory was not found!")
        }

        WalkDir::new(root)
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
                    Ok(DriverItem {
                        path: get_relative_utf8_path(path, root)?.to_string(),
                        metadata: DriverItemMetadata::Directory,
                    })
                } else if path.is_file() {
                    let metadata = path.metadata().with_context(|| {
                        format!("Failed to get file's metadata for: {}", path.display())
                    })?;

                    // TODO: get real size
                    Ok(DriverItem {
                        path: get_relative_utf8_path(path, root)?.to_string(),
                        metadata: DriverItemMetadata::File(DriverFileMetadata {
                            creation_date: metadata.ctime(),
                            modification_date: metadata.mtime(),
                            size: metadata.len(),
                        }),
                    })
                } else {
                    bail!("Encountered unknown item type at: {}", path.display())
                }
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

fn get_relative_utf8_path<'a>(path: &'a Path, source: &Path) -> Result<&'a str> {
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
