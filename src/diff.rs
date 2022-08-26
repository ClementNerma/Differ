use crate::snapshot::{Snapshot, SnapshotItemMetadata};
use std::{collections::HashSet, path::PathBuf, cmp::Ordering};

pub struct Diff(Vec<DiffItem>);

impl Diff {
    pub fn new(items: Vec<DiffItem>) -> Self {
        Self(items)
    }

    pub fn items(&self) -> &[DiffItem] {
        self.0.as_slice()
    }

    // pub fn into_items(self) -> Vec<DiffItem> {
    //     self.0
    // }

    pub fn sort(&mut self) {
        self.0.sort()
    }
}

#[derive(PartialEq, Eq)]
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
        self.status.cmp(&other.status).then_with(|| self.path.cmp(&other.path)).then(Ordering::Equal)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffType {
    Added,
    Changed,
    TypeChanged, // File => Dir / Dir => File
    Deleted,
}

pub fn build_diff(source: Snapshot, backup_dir: Snapshot) -> Diff {
    let source_items = build_item_names_hashset(&source);
    let backed_up_items = build_item_names_hashset(&backup_dir);

    let mut diff = Vec::with_capacity(source_items.len());

    diff.extend(
        source_items
            .difference(&backed_up_items)
            .map(|item| DiffItem {
                path: PathBuf::from(item),
                status: DiffType::Added,
            }),
    );

    diff.extend(
        backed_up_items
            .difference(&source_items)
            .map(|item| DiffItem {
                path: PathBuf::from(item),
                status: DiffType::Deleted,
            }),
    );

    diff.extend(
        source
            .items
            .iter()
            .filter(|item| backed_up_items.contains(&item.path))
            .filter_map(|source_item| {
                let backed_up_item = backup_dir
                    .items
                    .iter()
                    .find(|c| c.path == source_item.path)
                    .unwrap();

                match (&source_item.metadata, &backed_up_item.metadata) {
                    // Both directories = no change
                    (SnapshotItemMetadata::Directory, SnapshotItemMetadata::Directory) => None,
                    // Source item is directory and backed up item is file or the opposite = type changed
                    (SnapshotItemMetadata::Directory, SnapshotItemMetadata::File { .. })
                    | (SnapshotItemMetadata::File { .. }, SnapshotItemMetadata::Directory) => {
                        Some(DiffItem {
                            path: PathBuf::from(&source_item.path),
                            status: DiffType::TypeChanged,
                        })
                    }
                    // Otherwise, compare their metadata to see if something changed
                    (
                        SnapshotItemMetadata::File {
                            comparable: source_data,
                            ..
                        },
                        SnapshotItemMetadata::File {
                            comparable: backed_up_data,
                            ..
                        },
                    ) => {
                        if source_data == backed_up_data {
                            None
                        } else {
                            Some(DiffItem {
                                path: PathBuf::from(&source_item.path),
                                status: DiffType::Changed,
                            })
                        }
                    }
                }
            }),
    );

    let mut diff = Diff::new(diff);
    diff.sort();
    diff
}

fn build_item_names_hashset(snapshot: &Snapshot) -> HashSet<&String> {
    snapshot
        .items
        .iter()
        .map(|item| &item.path)
        .collect::<HashSet<_>>()
}
