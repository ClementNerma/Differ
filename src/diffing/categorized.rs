use super::{
    Diff, DiffItemAdded, DiffItemDeleted, DiffItemModified, DiffItemTypeChanged, DiffType,
};

pub struct CategorizedDiff {
    pub added: Vec<(String, DiffItemAdded)>,
    pub modified: Vec<(String, DiffItemModified)>,
    pub type_changed: Vec<(String, DiffItemTypeChanged)>,
    pub deleted: Vec<(String, DiffItemDeleted)>,
}

impl CategorizedDiff {
    pub fn new(diff: Diff) -> Self {
        let mut added = vec![];
        let mut modified = vec![];
        let mut type_changed = vec![];
        let mut deleted = vec![];

        for item in diff.into_items() {
            match item.status {
                DiffType::Added(i) => added.push((item.path, i)),
                DiffType::Modified(i) => modified.push((item.path, i)),
                DiffType::TypeChanged(i) => type_changed.push((item.path, i)),
                DiffType::Deleted(i) => deleted.push((item.path, i)),
            }
        }

        Self {
            added,
            modified,
            type_changed,
            deleted,
        }
    }
}
