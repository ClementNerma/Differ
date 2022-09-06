use std::collections::HashSet;

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Snapshot {
    // TODO: add checksum
    // TODO: add creation date
    pub driver_id: String,
    pub path: String,
    pub items: Vec<DriverItem>,
}

pub fn make_snapshot(driver: &dyn Driver, path: &str) -> Result<Snapshot> {
    let path = driver.canonicalize(path)?;

    let items = driver.find_all(&path)?;

    let mut uniq = HashSet::new();

    for item in &items {
        if !uniq.insert(&item.path) {
            bail!("Duplicate item in driver's results: {}", item.path);
        }
    }

    Ok(Snapshot {
        driver_id: driver.id(),
        items,
        path,
    })
}

pub trait Driver {
    // fn get_metadata(path: &Path) -> Result<ItemMetadata, DriverError>;
    fn id(&self) -> String;
    fn find_all(&self, dir: &str) -> Result<Vec<DriverItem>>;
    fn canonicalize(&self, path: &str) -> Result<String>;
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

    pub fn is_file(&self) -> bool {
        match self {
            Self::Directory => false,
            Self::File(_) => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DriverFileMetadata {
    // pub creation_date: i64,
    pub modification_date: i64,
    pub size: u64,
}
