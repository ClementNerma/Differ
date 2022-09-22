use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Snapshot {
    // TODO: add checksum
    // TODO: add creation date
    pub path: String,
    pub items: Vec<DriverItem>,
}

pub fn make_snapshot(
    driver: &dyn Driver,
    path: String,
    ignore: &HashSet<&str>,
    stop_request: Arc<AtomicBool>,
    on_item: Option<OnItemHandler>,
) -> Result<Snapshot> {
    let items = driver.find_all(&path, ignore, Arc::clone(&stop_request), on_item);

    // TODO: When https://github.com/rust-lang/rust/issues/91345 is resolved, use `inspect_err` instead of a match
    let items = match items {
        Ok(items) => items,
        Err(e) => {
            stop_request.store(true, Ordering::Relaxed);
            return Err(e);
        }
    };

    let mut uniq = HashSet::new();

    for item in &items {
        if !uniq.insert(&item.path) {
            bail!("Duplicate item in driver's results: {}", item.path);
        }
    }

    Ok(Snapshot { items, path })
}

pub trait Driver {
    fn find_all(
        &self,
        dir: &str,
        ignore: &HashSet<&str>,
        stop_request: Arc<AtomicBool>,
        on_item: Option<OnItemHandler>,
    ) -> Result<Vec<DriverItem>>;
}

pub type OnItemHandler = Box<dyn Fn(&DriverItem) + Send + Sync + 'static>;

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
