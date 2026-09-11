//! Milestone 001's two-peer convergence test (see `docs/milestone-001.md`
//! and `README.md`'s "First milestone definition"), exercised across both
//! `samgtd-crdt` (documents) and `samgtd-store` (SQLite persistence).
//!
//! Shared state is established through native sync. Offline peers edit the
//! same task (including a same-field conflict) and create a second task.
//! They restart while disconnected, reconnect, compare full fields and heads,
//! reload both SQLite files, and repeat sync.

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
        category_id: "00000000-0000-4000-8000-000000000001".to_string(),
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
    /// The seed preserves shared root map history. This fixture uses genesis;
    /// real provisioning can transfer the current root after any number of edits.
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
        let mut blobs = vec![(ROOT_INDEX_TYPE, ROOT_INDEX_ID.to_owned(), self.root.save())];
        for (uuid, doc) in self.tasks.iter_mut() {
            blobs.push((TASK_TYPE, uuid.clone(), doc.save()));
        }
        let batch: Vec<_> = blobs
            .iter()
            .map(|(kind, id, bytes)| (*kind, id.as_str(), bytes.as_slice()))
            .collect();
        self.store.save_documents(&batch).unwrap();
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
    for round in 0..100 {
        assert!(round < 99, "sync did not settle");
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
        a.tasks
            .entry(uuid.clone())
            .or_insert_with(TaskDocument::empty);
        b.tasks
            .entry(uuid.clone())
            .or_insert_with(TaskDocument::empty);

        let mut state_a = sync::State::new();
        let mut state_b = sync::State::new();
        for round in 0..100 {
            assert!(round < 99, "sync did not settle");
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

    // Establish shared state using only native sync messages for task transfer.
    peer_a.create_task("00000000-0000-4000-8000-000000000002", "Buy milk");
    sync_root(&mut peer_a, &mut peer_b);
    sync_tasks(&mut peer_a, &mut peer_b);

    // Disconnected replicas edit the SAME task, including a same-field conflict.
    peer_a
        .tasks
        .get_mut("00000000-0000-4000-8000-000000000002")
        .unwrap()
        .set_status("DONE")
        .unwrap();
    peer_a
        .tasks
        .get_mut("00000000-0000-4000-8000-000000000002")
        .unwrap()
        .set_notes("A notes")
        .unwrap();
    peer_b
        .tasks
        .get_mut("00000000-0000-4000-8000-000000000002")
        .unwrap()
        .set_title("Buy oat milk")
        .unwrap();
    peer_b
        .tasks
        .get_mut("00000000-0000-4000-8000-000000000002")
        .unwrap()
        .set_notes("B notes")
        .unwrap();
    peer_b.create_task("00000000-0000-4000-8000-000000000003", "File taxes");
    peer_a.persist();
    peer_b.persist();

    // Restart while disconnected, then establish fresh per-document sync states.
    drop(peer_a);
    drop(peer_b);
    let mut peer_a = Peer::open(&path_a, &genesis_root_bytes);
    let mut peer_b = Peer::open(&path_b, &genesis_root_bytes);
    sync_root(&mut peer_a, &mut peer_b);
    sync_tasks(&mut peer_a, &mut peer_b);
    assert_equivalent(&mut peer_a, &mut peer_b);
    let expected = peer_a.tasks["00000000-0000-4000-8000-000000000002"]
        .fields()
        .unwrap();
    assert_eq!(expected.title, "Buy oat milk");
    assert_eq!(expected.status, "DONE");
    assert!(matches!(expected.notes.as_str(), "A notes" | "B notes"));

    // 5 & 6: restart both processes and reload converged state from disk.
    drop(peer_a);
    drop(peer_b);
    let mut peer_a = Peer::open(&path_a, &genesis_root_bytes);
    let mut peer_b = Peer::open(&path_b, &genesis_root_bytes);

    assert_eq!(
        peer_a.task_uuids(),
        vec![
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000003"
        ]
    );
    assert_eq!(
        peer_b.task_uuids(),
        vec![
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000003"
        ]
    );
    assert_eq!(
        peer_a.tasks["00000000-0000-4000-8000-000000000002"]
            .fields()
            .unwrap(),
        expected
    );
    assert_equivalent(&mut peer_a, &mut peer_b);
    assert_eq!(
        peer_b.tasks["00000000-0000-4000-8000-000000000003"]
            .fields()
            .unwrap()
            .title,
        "File taxes"
    );

    // 7: repeat sync must not lose or duplicate entities.
    sync_root(&mut peer_a, &mut peer_b);
    sync_tasks(&mut peer_a, &mut peer_b);

    assert_eq!(
        peer_a.task_uuids(),
        vec![
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000003"
        ]
    );
    assert_eq!(
        peer_b.task_uuids(),
        vec![
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000003"
        ]
    );
    assert_equivalent(&mut peer_a, &mut peer_b);
}

fn assert_equivalent(a: &mut Peer, b: &mut Peer) {
    assert_eq!(a.task_uuids(), b.task_uuids());
    assert_eq!(a.root.heads(), b.root.heads());
    for id in a.task_uuids() {
        assert_eq!(
            a.tasks[&id].fields().unwrap(),
            b.tasks[&id].fields().unwrap()
        );
        assert_eq!(
            a.tasks.get_mut(&id).unwrap().heads(),
            b.tasks.get_mut(&id).unwrap().heads()
        );
    }
}
