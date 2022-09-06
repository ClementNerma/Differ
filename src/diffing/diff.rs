use crate::drivers::{DriverFileMetadata, DriverItemMetadata, Snapshot};

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub struct Diff(Vec<DiffItem>);

impl Diff {
    pub fn new(items: Vec<DiffItem>) -> Self {
        Self(items)
    }

    pub fn items(&self) -> &[DiffItem] {
        self.0.as_slice()
    }

    pub fn into_items(self) -> Vec<DiffItem> {
        self.0
    }

    // pub fn into_items(self) -> Vec<DiffItem> {
    //     self.0
    // }

    pub fn sort(&mut self) {
        self.0.sort()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiffItem {
    pub path: PathBuf,
    pub status: DiffType,
}

impl PartialOrd for DiffItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiffItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.status
            .cmp(&other.status)
            .then_with(|| self.path.cmp(&other.path))
            .then(Ordering::Equal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffType {
    Added(DiffItemAdded),
    Modified(DiffItemModified),
    TypeChanged(DiffItemTypeChanged), // File => Dir / Dir => File
    Deleted(DiffItemDeleted),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiffItemAdded {
    pub new: DriverItemMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiffItemModified {
    pub prev: DriverFileMetadata,
    pub new: DriverFileMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiffItemTypeChanged {
    pub prev: DriverItemMetadata,
    pub new: DriverItemMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiffItemDeleted {
    pub prev: DriverItemMetadata,
}

impl DiffType {
    pub fn get_new_metadata(self) -> Option<DriverItemMetadata> {
        match self {
            Self::Added(DiffItemAdded { new }) => Some(new),
            Self::Modified(DiffItemModified { prev: _, new }) => {
                Some(DriverItemMetadata::File(new))
            }
            Self::TypeChanged(DiffItemTypeChanged { prev: _, new }) => Some(new),
            Self::Deleted(DiffItemDeleted { prev: _ }) => None,
        }
    }

    pub fn get_prev_metadata(self) -> Option<DriverItemMetadata> {
        match self {
            Self::Added(DiffItemAdded { new: _ }) => None,
            Self::Modified(DiffItemModified { prev, new: _ }) => {
                Some(DriverItemMetadata::File(prev))
            }
            Self::TypeChanged(DiffItemTypeChanged { prev, new: _ }) => Some(prev),
            Self::Deleted(DiffItemDeleted { prev }) => Some(prev),
        }
    }

    pub fn get_relevant_metadata(self) -> DriverItemMetadata {
        match self {
            Self::Added(DiffItemAdded { new }) => new,
            Self::Modified(DiffItemModified { prev: _, new }) => DriverItemMetadata::File(new),
            Self::TypeChanged(DiffItemTypeChanged { prev: _, new }) => new,
            Self::Deleted(DiffItemDeleted { prev }) => prev,
        }
    }
}

pub fn build_diff(source: Snapshot, backup_dir: Snapshot) -> Diff {
    let source_items = build_item_names_hashmap(&source);
    let backed_up_items = build_item_names_hashmap(&backup_dir);

    let source_items_paths: HashSet<_> = source_items.keys().collect();
    let backed_up_items_paths: HashSet<_> = backed_up_items.keys().collect();

    let mut diff = Vec::with_capacity(source_items.len());

    diff.extend(
        source_items_paths
            .difference(&backed_up_items_paths)
            .map(|item| DiffItem {
                path: PathBuf::from(item),
                status: DiffType::Added(DiffItemAdded {
                    new: **source_items.get(*item).unwrap(),
                }),
            }),
    );

    diff.extend(
        backed_up_items_paths
            .difference(&source_items_paths)
            .map(|item| DiffItem {
                path: PathBuf::from(item),
                status: DiffType::Deleted(DiffItemDeleted {
                    prev: **backed_up_items.get(*item).unwrap(),
                }),
            }),
    );

    diff.extend(
        source
            .items
            .iter()
            .filter(|item| backed_up_items_paths.contains(&&item.path))
            .filter_map(|source_item| {
                let backed_up_item = backup_dir
                    .items
                    .iter()
                    .find(|c| c.path == source_item.path)
                    .unwrap();

                match (source_item.metadata, backed_up_item.metadata) {
                    // Both directories = no change
                    (DriverItemMetadata::Directory, DriverItemMetadata::Directory) => None,
                    // Source item is directory and backed up item is file or the opposite = type changed
                    (DriverItemMetadata::Directory, DriverItemMetadata::File { .. })
                    | (DriverItemMetadata::File { .. }, DriverItemMetadata::Directory) => {
                        Some(DiffItem {
                            path: PathBuf::from(&source_item.path),
                            status: DiffType::TypeChanged(DiffItemTypeChanged {
                                prev: backed_up_item.metadata,
                                new: source_item.metadata,
                            }),
                        })
                    }
                    // Otherwise, compare their metadata to see if something changed
                    (
                        DriverItemMetadata::File(source_data),
                        DriverItemMetadata::File(backed_up_data),
                    ) => {
                        if source_data == backed_up_data {
                            None
                        } else {
                            Some(DiffItem {
                                path: PathBuf::from(&source_item.path),
                                status: DiffType::Modified(DiffItemModified {
                                    prev: backed_up_data,
                                    new: source_data,
                                }),
                            })
                        }
                    }
                }
            }),
    );

    Diff::new(diff)
}

fn build_item_names_hashmap(snapshot: &Snapshot) -> HashMap<&String, &DriverItemMetadata> {
    snapshot
        .items
        .iter()
        .map(|item| (&item.path, &item.metadata))
        .collect::<HashMap<_, _>>()
}
