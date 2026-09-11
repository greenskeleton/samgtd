//! Wires `samgtd-store` (SQLite) and `samgtd-crdt` (Automerge) together for
//! daemon startup: load the root index document if one exists on disk, or
//! create a fresh dataset if this is the first run.

use samgtd_crdt::RootIndexDocument;
use samgtd_store::Store;
use std::path::Path;

const ROOT_INDEX_TYPE: &str = "root_index";
const ROOT_INDEX_ID: &str = "default";

/// Open (creating if needed) the daemon's database, and load or create the
/// root index document, persisting it back so a fresh database always ends
/// up with one on disk.
pub fn init_persistence(db_path: &Path) -> anyhow::Result<RootIndexDocument> {
    let store = Store::open(db_path)?;

    let mut root = match store.load_document(ROOT_INDEX_TYPE, ROOT_INDEX_ID)? {
        Some(bytes) => RootIndexDocument::load(&bytes)?,
        None => RootIndexDocument::new()?,
    };

    store.save_document(ROOT_INDEX_TYPE, ROOT_INDEX_ID, &root.save())?;

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_run_loads_the_same_dataset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("samgtd.sqlite");

        let mut first = init_persistence(&path).unwrap();
        first.add_task("t1").unwrap();
        let store = Store::open(&path).unwrap();
        store
            .save_document(ROOT_INDEX_TYPE, ROOT_INDEX_ID, &first.save())
            .unwrap();

        let second = init_persistence(&path).unwrap();
        assert_eq!(second.task_uuids().unwrap(), vec!["t1"]);
    }
}
