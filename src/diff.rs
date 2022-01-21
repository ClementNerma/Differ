use crate::snapshot::{Snapshot, SnapshotItem, SnapshotItemMetadata};
use std::{cmp::Ordering, collections::HashSet, path::PathBuf};

pub type Diff = Vec<DiffItem>;

#[derive(PartialEq, Eq)]
pub struct DiffItem {
    pub path: PathBuf,
    pub status: DiffType,
}

impl PartialOrd for DiffItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.status.cmp(&other.status) {
            Ordering::Equal => self.path.partial_cmp(&other.path),
            ord => Some(ord),
        }
    }
}

impl Ord for DiffItem {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.status.cmp(&other.status) {
            Ordering::Equal => self.path.cmp(&other.path),
            ord => ord,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffType {
    Added = 1,
    Changed = 2,
    TypeChanged = 3, // File => Dir / Dir => File
    Deleted = 4,
}

pub fn build_diff(source: Snapshot, dest: Snapshot) -> Diff {
    let source_items = build_item_names_hashset(&source);
    let dest_items = build_item_names_hashset(&dest);

    let mut diff = Vec::with_capacity(source_items.len());

    diff.extend(source_items.difference(&dest_items).map(|item| DiffItem {
        path: PathBuf::from(item),
        status: DiffType::Added,
    }));

    diff.extend(dest_items.difference(&source_items).map(|item| DiffItem {
        path: PathBuf::from(item),
        status: DiffType::Deleted,
    }));

    diff.extend(
        source
            .items
            .iter()
            .filter(|item| dest_items.contains(&item.path))
            .filter_map(|source_item| {
                let dest_item = dest
                    .items
                    .iter()
                    .find(|c| c.path == source_item.path)
                    .unwrap();

                match (&source_item.metadata, &dest_item.metadata) {
                    // Both directories = no change
                    (SnapshotItemMetadata::Directory, SnapshotItemMetadata::Directory) => None,
                    // Source item is directory and destination item is file or the opposite = type changed
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
                            comparable: dest_data,
                            ..
                        },
                    ) => {
                        if source_data == dest_data {
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
