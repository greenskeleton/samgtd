//! Read-only, post-mortem inspection of a synthetic peer's SQLite store.
//!
//! The public HTTP API deliberately has no document-dump/heads endpoint
//! (adding one just for tests is out of scope for this milestone). Instead,
//! once a daemon process has fully exited — releasing the store's advisory
//! lock — this opens the same store the daemon was using and reads
//! documents back through the existing `samgtd-store`/`samgtd-crdt` crates,
//! the same libraries the daemon itself uses. This never runs against a
//! live daemon's store and is never used to shuttle data between replicas;
//! it only verifies, after the fact, what each replica actually persisted.

use anyhow::Context;
use samgtd_crdt::{RootIndexDocument, TaskDocument, TaskFields};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSummary {
    pub task_uuids: Vec<String>,
    pub root_heads: Vec<String>,
    pub task_heads: BTreeMap<String, Vec<String>>,
    pub task_fields: BTreeMap<String, TaskFields>,
}

fn sorted_heads(mut heads: Vec<automerge::ChangeHash>) -> Vec<String> {
    let mut hex: Vec<String> = heads.drain(..).map(|h| h.to_string()).collect();
    hex.sort();
    hex
}

/// Open `db_path` (the daemon holding it must already have exited) and
/// summarize the root index and every indexed task document.
pub fn inspect(db_path: &Path) -> anyhow::Result<DocumentSummary> {
    let store = samgtd_store::Store::open(db_path)
        .with_context(|| format!("open {} for read-only inspection", db_path.display()))?;
    let root_bytes = store
        .load_document("root_index", "default")?
        .context("store has no root_index/default document")?;
    let mut root = RootIndexDocument::load(&root_bytes)?;
    let mut task_uuids = root.task_uuids()?;
    task_uuids.sort();
    let root_heads = sorted_heads(root.heads());

    let mut task_heads = BTreeMap::new();
    let mut task_fields = BTreeMap::new();
    for id in &task_uuids {
        let bytes = store
            .load_document("task", id)?
            .with_context(|| format!("store is missing indexed task {id}"))?;
        let mut doc = TaskDocument::load(&bytes)?;
        task_heads.insert(id.clone(), sorted_heads(doc.heads()));
        task_fields.insert(id.clone(), doc.fields()?);
    }

    Ok(DocumentSummary {
        task_uuids,
        root_heads,
        task_heads,
        task_fields,
    })
}
