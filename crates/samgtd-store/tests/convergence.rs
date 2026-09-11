//! Milestone 001's two-peer convergence test (see `docs/milestone-001.md`
//! and `README.md`'s "First milestone definition"), exercised across both
//! `samgtd-crdt` (documents) and `samgtd-store` (SQLite persistence).
//!
//! Scenario:
//! 1. Peer A creates a task while peer B is disconnected.
//! 2. Peer B, independently and concurrently, creates a different task.
//! 3. Reconnect: sync the root index and both task documents.
//! 4. Converge to equivalent state (both peers know both tasks).
//! 5. Restart both processes (drop and reopen `Store` from the same file).
//! 6. Load the converged state from disk.
//! 7. Repeat sync without losing or duplicating entities.

use automerge::sync;
use samgtd_crdt::{RootIndexDocument, TaskDocument, TaskFields};
use samgtd_store::Store;
use std::path::Path;

const ROOT_INDEX_TYPE: &str = "root_index";
const ROOT_INDEX_ID: &str = "default";
const TASK_TYPE: &str = "task";

fn task_fields(uuid: &str, title: &str) -> TaskFields {
    TaskFields {
        uuid: uuid.to_string(),
        title: title.to_string(),
        notes: String::new(),
        status: "TODO".to_string(),
        category_id: "cat-inbox".to_string(),
        project_id: None,
        domain_id: None,
    }
}

/// A peer's on-disk state: a `Store` plus the root index + task documents it
/// currently has loaded.
struct Peer {
    store: Store,
    root: RootIndexDocument,
    tasks: std::collections::BTreeMap<String, TaskDocument>,
}

impl Peer {
    /// Open (creating if needed) a peer's store and load whatever documents
    /// already exist on disk.
    ///
    /// `genesis_root_bytes` seeds a brand-new store. This must be the *same*
    /// bytes for every peer of a dataset: two independently `::new()`-created
    /// `RootIndexDocument`s do not merge safely even though they use the same
    /// key names, because each `put_object` mints a fresh object ID from its
    /// own actor. Automerge sync/merge requires a shared ancestor — in
    /// practice, a dataset is created once and every peer is provisioned
    /// from that same initial document (e.g. device pairing), never by
    /// independently calling `RootIndexDocument::new()`.
    fn open(path: &Path, genesis_root_bytes: &[u8]) -> Self {
        let store = Store::open(path).unwrap();

        let root = match store.load_document(ROOT_INDEX_TYPE, ROOT_INDEX_ID).unwrap() {
            Some(bytes) => RootIndexDocument::load(&bytes).unwrap(),
            None => RootIndexDocument::load(genesis_root_bytes).unwrap(),
        };

        let mut tasks = std::collections::BTreeMap::new();
        for uuid in store.list_document_ids(TASK_TYPE).unwrap() {
            let bytes = store.load_document(TASK_TYPE, &uuid).unwrap().unwrap();
            tasks.insert(uuid, TaskDocument::load(&bytes).unwrap());
        }

        Self { store, root, tasks }
    }

    fn create_task(&mut self, uuid: &str, title: &str) {
        let doc = TaskDocument::new(&task_fields(uuid, title)).unwrap();
        self.tasks.insert(uuid.to_string(), doc);
        self.root.add_task(uuid).unwrap();
        self.persist();
    }

    /// Save every in-memory document back to this peer's store.
    fn persist(&mut self) {
        let root_bytes = self.root.save();
        self.store
            .save_document(ROOT_INDEX_TYPE, ROOT_INDEX_ID, &root_bytes)
            .unwrap();
        for (uuid, doc) in self.tasks.iter_mut() {
            let bytes = doc.save();
            self.store.save_document(TASK_TYPE, uuid, &bytes).unwrap();
        }
    }

    fn task_uuids(&self) -> Vec<String> {
        let mut ids = self.root.task_uuids().unwrap();
        ids.sort();
        ids
    }
}

/// Sync two peers' root index documents until neither has anything left to
/// send.
fn sync_root(a: &mut Peer, b: &mut Peer) {
    let mut state_a = sync::State::new();
    let mut state_b = sync::State::new();
    loop {
        let mut progressed = false;
        if let Some(msg) = a.root.generate_sync_message(&mut state_a) {
            b.root.receive_sync_message(&mut state_b, &msg).unwrap();
            progressed = true;
        }
        if let Some(msg) = b.root.generate_sync_message(&mut state_b) {
            a.root.receive_sync_message(&mut state_a, &msg).unwrap();
            progressed = true;
        }
        if !progressed {
            break;
        }
    }
}

/// After the root index is synced, both peers know about the union of task
/// UUIDs. Bring each peer's task document set up to date: for any UUID a
/// peer doesn't yet have, initialize an empty placeholder and sync it in
/// from whichever peer already has it (a real daemon would request the
/// document directly; this test drives the same sync primitive).
fn sync_tasks(a: &mut Peer, b: &mut Peer) {
    let all_uuids: std::collections::BTreeSet<String> =
        a.task_uuids().into_iter().chain(b.task_uuids()).collect();

    for uuid in all_uuids {
        let a_has = a.tasks.contains_key(&uuid);
        let b_has = b.tasks.contains_key(&uuid);

        match (a_has, b_has) {
            (true, true) | (false, false) => {}
            (true, false) => {
                let bytes = a.tasks.get_mut(&uuid).unwrap().save();
                b.tasks
                    .insert(uuid.clone(), TaskDocument::load(&bytes).unwrap());
            }
            (false, true) => {
                let bytes = b.tasks.get_mut(&uuid).unwrap().save();
                a.tasks
                    .insert(uuid.clone(), TaskDocument::load(&bytes).unwrap());
            }
        }

        let mut state_a = sync::State::new();
        let mut state_b = sync::State::new();
        loop {
            let mut progressed = false;
            let doc_a = a.tasks.get_mut(&uuid).unwrap();
            if let Some(msg) = doc_a.generate_sync_message(&mut state_a) {
                b.tasks
                    .get_mut(&uuid)
                    .unwrap()
                    .receive_sync_message(&mut state_b, &msg)
                    .unwrap();
                progressed = true;
            }
            let doc_b = b.tasks.get_mut(&uuid).unwrap();
            if let Some(msg) = doc_b.generate_sync_message(&mut state_b) {
                a.tasks
                    .get_mut(&uuid)
                    .unwrap()
                    .receive_sync_message(&mut state_a, &msg)
                    .unwrap();
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
    }

    a.persist();
    b.persist();
}

#[test]
fn two_peers_converge_survive_restart_and_resync_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let path_a = dir.path().join("peer-a.sqlite");
    let path_b = dir.path().join("peer-b.sqlite");

    let genesis_root_bytes = RootIndexDocument::new().unwrap().save();
    let mut peer_a = Peer::open(&path_a, &genesis_root_bytes);
    let mut peer_b = Peer::open(&path_b, &genesis_root_bytes);

    // 1 & 2: each peer creates a different task while "disconnected" from
    // the other (they never exchange a message until step 3).
    peer_a.create_task("task-a", "Buy milk");
    peer_b.create_task("task-b", "File taxes");

    // 3 & 4: reconnect, sync, converge.
    sync_root(&mut peer_a, &mut peer_b);
    sync_tasks(&mut peer_a, &mut peer_b);

    assert_eq!(peer_a.task_uuids(), vec!["task-a", "task-b"]);
    assert_eq!(peer_a.task_uuids(), peer_b.task_uuids());

    // 5 & 6: restart both processes and reload converged state from disk.
    drop(peer_a);
    drop(peer_b);
    let mut peer_a = Peer::open(&path_a, &genesis_root_bytes);
    let mut peer_b = Peer::open(&path_b, &genesis_root_bytes);

    assert_eq!(peer_a.task_uuids(), vec!["task-a", "task-b"]);
    assert_eq!(peer_b.task_uuids(), vec!["task-a", "task-b"]);
    assert_eq!(peer_a.tasks["task-a"].fields().unwrap().title, "Buy milk");
    assert_eq!(peer_b.tasks["task-b"].fields().unwrap().title, "File taxes");

    // 7: repeat sync must not lose or duplicate entities.
    sync_root(&mut peer_a, &mut peer_b);
    sync_tasks(&mut peer_a, &mut peer_b);

    assert_eq!(peer_a.task_uuids(), vec!["task-a", "task-b"]);
    assert_eq!(peer_b.task_uuids(), vec!["task-a", "task-b"]);
}
