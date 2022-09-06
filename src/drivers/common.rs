use std::collections::HashSet;

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Snapshot {
    // TODO: add checksum
    // TODO: add creation date
    pub path: String,
    pub items: Vec<DriverItem>,
}

pub fn make_snapshot(driver: &dyn Driver, path: String) -> Result<Snapshot> {
    let items = driver.find_all(&path)?;

    let mut uniq = HashSet::new();

    for item in &items {
        if !uniq.insert(&item.path) {
            bail!("Duplicate item in driver's results: {}", item.path);
        }
    }

    Ok(Snapshot { items, path })
}

pub trait Driver {
    fn find_all(&self, dir: &str) -> Result<Vec<DriverItem>>;
}

#[derive(Debug)]
pub struct DriverItem {
    pub path: String,
    pub metadata: DriverItemMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriverItemMetadata {
    Directory,
    File(DriverFileMetadata),
}

impl DriverItemMetadata {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DriverFileMetadata {
    // pub creation_date: i64,
    pub modification_date: i64,
    pub size: u64,
}
