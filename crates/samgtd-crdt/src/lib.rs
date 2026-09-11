#![forbid(unsafe_code)]

//! Automerge CRDT schema, serialization, and merge helpers.
//!
//! Document granularity follows `docs/adr/0003-existing-database-coexistence.md`:
//! a hybrid root/index document plus one document per entity. Only the Task
//! document is implemented so far (Milestone 001's vertical slice).

use automerge::sync::SyncDoc;
use automerge::{sync, transaction::Transactable, AutoCommit, ObjType, ReadDoc, ROOT};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrdtError {
    #[error("invalid task: {0}")]
    InvalidTask(#[from] samgtd_domain::TaskError),
    #[error("conflicting immutable identity or root schema")]
    ConflictingIdentity,
    #[error("automerge error: {0}")]
    Automerge(#[from] automerge::AutomergeError),
    #[error("failed to decode sync message: {0}")]
    DecodeSyncMessage(#[from] automerge::sync::ReadMessageError),
    #[error("required field {0:?} missing or wrong type")]
    MissingField(&'static str),
}

pub use samgtd_domain::TaskFields;

/// An Automerge document representing one Task entity.
pub struct TaskDocument {
    doc: AutoCommit,
}

impl TaskDocument {
    /// Empty receiver: no independently initialized schema objects or fields.
    pub fn empty() -> Self {
        Self {
            doc: AutoCommit::new(),
        }
    }

    pub fn heads(&mut self) -> Vec<automerge::ChangeHash> {
        self.doc.get_heads()
    }

    pub fn set_notes(&mut self, notes: &str) -> Result<(), CrdtError> {
        self.doc.put(ROOT, "notes", notes)?;
        Ok(())
    }

    /// Create a brand-new Task document.
    pub fn new(fields: &TaskFields) -> Result<Self, CrdtError> {
        fields.validate()?;
        let mut doc = AutoCommit::new();
        write_fields(&mut doc, fields)?;
        Ok(Self { doc })
    }

    /// Load a Task document from previously saved bytes.
    pub fn load(bytes: &[u8]) -> Result<Self, CrdtError> {
        let doc = AutoCommit::load(bytes)?;
        Ok(Self { doc })
    }

    /// Serialize the document to bytes for persistence.
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    pub fn fields(&self) -> Result<TaskFields, CrdtError> {
        read_fields(&self.doc)
    }

    pub fn set_title(&mut self, title: &str) -> Result<(), CrdtError> {
        self.doc.put(ROOT, "title", title)?;
        Ok(())
    }

    pub fn set_status(&mut self, status: &str) -> Result<(), CrdtError> {
        if !samgtd_domain::is_valid_status(status) {
            return Err(CrdtError::MissingField("status"));
        }
        self.doc.put(ROOT, "status", status)?;
        Ok(())
    }

    /// Generate the next sync message to send to `peer_state`'s peer, if any
    /// changes need to cross the wire.
    pub fn generate_sync_message(&mut self, peer_state: &mut sync::State) -> Option<Vec<u8>> {
        self.doc
            .sync()
            .generate_sync_message(peer_state)
            .map(|msg| msg.encode())
    }

    /// Apply a sync message received from a peer.
    pub fn receive_sync_message(
        &mut self,
        peer_state: &mut sync::State,
        message: &[u8],
    ) -> Result<(), CrdtError> {
        let message = sync::Message::decode(message)?;
        self.doc.sync().receive_sync_message(peer_state, message)?;
        Ok(())
    }

    /// Merge another in-memory document directly (used in tests; sync
    /// messages are the real wire path).
    pub fn merge(&mut self, other: &mut TaskDocument) -> Result<(), CrdtError> {
        self.doc.merge(&mut other.doc)?;
        Ok(())
    }
}

fn write_fields(doc: &mut AutoCommit, fields: &TaskFields) -> Result<(), CrdtError> {
    doc.put(ROOT, "uuid", fields.uuid.as_str())?;
    doc.put(ROOT, "title", fields.title.as_str())?;
    doc.put(ROOT, "notes", fields.notes.as_str())?;
    doc.put(ROOT, "status", fields.status.as_str())?;
    doc.put(ROOT, "category_id", fields.category_id.as_str())?;
    if let Some(v) = &fields.project_id {
        doc.put(ROOT, "project_id", v.as_str())?;
    }
    if let Some(v) = &fields.domain_id {
        doc.put(ROOT, "domain_id", v.as_str())?;
    }
    Ok(())
}

fn read_str(doc: &AutoCommit, key: &'static str) -> Result<String, CrdtError> {
    doc.get(ROOT, key)?
        .and_then(|(v, _)| v.to_str().map(str::to_string))
        .ok_or(CrdtError::MissingField(key))
}

fn read_opt_str(doc: &AutoCommit, key: &'static str) -> Result<Option<String>, CrdtError> {
    match doc.get(ROOT, key)? {
        None => Ok(None),
        Some((value, _)) => value
            .to_str()
            .map(|v| Some(v.to_owned()))
            .ok_or(CrdtError::MissingField(key)),
    }
}

fn read_fields(doc: &AutoCommit) -> Result<TaskFields, CrdtError> {
    if doc.get_all(ROOT, "uuid")?.len() != 1 {
        return Err(CrdtError::ConflictingIdentity);
    }
    let fields = TaskFields {
        uuid: read_str(doc, "uuid")?,
        title: read_str(doc, "title")?,
        notes: read_str(doc, "notes")?,
        status: read_str(doc, "status")?,
        category_id: read_str(doc, "category_id")?,
        project_id: read_opt_str(doc, "project_id")?,
        domain_id: read_opt_str(doc, "domain_id")?,
    };
    fields.validate()?;
    Ok(fields)
}

/// The root/index document: tracks which task UUIDs exist in the dataset.
/// Concurrent inserts by different peers merge without duplication because
/// Automerge map keys are unique by UUID.
pub struct RootIndexDocument {
    doc: AutoCommit,
}

impl RootIndexDocument {
    pub fn heads(&mut self) -> Vec<automerge::ChangeHash> {
        self.doc.get_heads()
    }

    pub fn new() -> Result<Self, CrdtError> {
        let mut doc = AutoCommit::new();
        doc.put_object(ROOT, "tasks", ObjType::Map)?;
        Ok(Self { doc })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, CrdtError> {
        let doc = AutoCommit::load(bytes)?;
        Ok(Self { doc })
    }

    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Register a task UUID as known (the value is unused today; presence of
    /// the key is what matters for the convergence test).
    pub fn add_task(&mut self, task_uuid: &str) -> Result<(), CrdtError> {
        let tasks_id = self.tasks_obj_id()?;
        self.doc.put(&tasks_id, task_uuid, true)?;
        Ok(())
    }

    pub fn task_uuids(&self) -> Result<Vec<String>, CrdtError> {
        let tasks_id = self.tasks_obj_id()?;
        Ok(self.doc.keys(&tasks_id).collect())
    }

    pub fn generate_sync_message(&mut self, peer_state: &mut sync::State) -> Option<Vec<u8>> {
        self.doc
            .sync()
            .generate_sync_message(peer_state)
            .map(|msg| msg.encode())
    }

    pub fn receive_sync_message(
        &mut self,
        peer_state: &mut sync::State,
        message: &[u8],
    ) -> Result<(), CrdtError> {
        let message = sync::Message::decode(message)?;
        self.doc.sync().receive_sync_message(peer_state, message)?;
        Ok(())
    }

    fn tasks_obj_id(&self) -> Result<automerge::ObjId, CrdtError> {
        if self.doc.get_all(ROOT, "tasks")?.len() != 1 {
            return Err(CrdtError::ConflictingIdentity);
        }
        match self.doc.get(ROOT, "tasks")? {
            Some((automerge::Value::Object(ObjType::Map), id)) => Ok(id),
            _ => Err(CrdtError::MissingField("tasks")),
        }
    }
}

impl Default for RootIndexDocument {
    fn default() -> Self {
        Self::new().expect("creating an empty document cannot fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fields(uuid: &str, title: &str) -> TaskFields {
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

    #[test]
    fn independently_initialized_roots_are_rejected() {
        let mut a = RootIndexDocument::new().unwrap();
        let mut b = RootIndexDocument::new().unwrap();
        a.doc.merge(&mut b.doc).unwrap();
        assert!(matches!(
            a.task_uuids(),
            Err(CrdtError::ConflictingIdentity)
        ));
    }

    #[test]
    fn invalid_status_and_optional_types_are_rejected() {
        let mut doc = TaskDocument::new(&sample_fields(
            "00000000-0000-4000-8000-000000000004",
            "Task",
        ))
        .unwrap();
        assert!(doc.set_status("invalid").is_err());
        assert_eq!(doc.fields().unwrap().status, "TODO");
        doc.doc.put(ROOT, "project_id", 42_i64).unwrap();
        assert!(doc.fields().is_err());
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let mut doc = TaskDocument::new(&sample_fields(
            "00000000-0000-4000-8000-000000000004",
            "Buy milk",
        ))
        .unwrap();
        let bytes = doc.save();
        let loaded = TaskDocument::load(&bytes).unwrap();
        assert_eq!(
            loaded.fields().unwrap(),
            sample_fields("00000000-0000-4000-8000-000000000004", "Buy milk")
        );
    }

    #[test]
    fn concurrent_field_edits_converge_via_sync() {
        let mut peer_a = TaskDocument::new(&sample_fields(
            "00000000-0000-4000-8000-000000000004",
            "Buy milk",
        ))
        .unwrap();
        let mut peer_b = TaskDocument::load(&peer_a.save()).unwrap();

        // Offline concurrent edits.
        peer_a.set_status("DONE").unwrap();
        peer_b.set_title("Buy oat milk").unwrap();

        // Reconnect: exchange sync messages until both report nothing left.
        let mut state_a = sync::State::new();
        let mut state_b = sync::State::new();
        for round in 0..100 {
            assert!(round < 99, "sync did not settle");
            let mut progressed = false;
            if let Some(msg) = peer_a.generate_sync_message(&mut state_a) {
                peer_b.receive_sync_message(&mut state_b, &msg).unwrap();
                progressed = true;
            }
            if let Some(msg) = peer_b.generate_sync_message(&mut state_b) {
                peer_a.receive_sync_message(&mut state_a, &msg).unwrap();
                progressed = true;
            }
            if !progressed {
                break;
            }
        }

        let a = peer_a.fields().unwrap();
        let b = peer_b.fields().unwrap();
        assert_eq!(a, b, "peers must converge to identical state");
        assert_eq!(a.status, "DONE");
        assert_eq!(a.title, "Buy oat milk");

        // Repeat sync is a safe no-op.
        assert!(peer_a.generate_sync_message(&mut state_a).is_none());
        assert!(peer_b.generate_sync_message(&mut state_b).is_none());
    }

    #[test]
    fn root_index_merges_concurrent_inserts_without_duplication() {
        let mut peer_a = RootIndexDocument::new().unwrap();
        let mut peer_b = RootIndexDocument::load(&peer_a.save()).unwrap();

        peer_a
            .add_task("00000000-0000-4000-8000-000000000002")
            .unwrap();
        peer_b
            .add_task("00000000-0000-4000-8000-000000000003")
            .unwrap();

        let mut state_a = sync::State::new();
        let mut state_b = sync::State::new();
        for round in 0..100 {
            assert!(round < 99, "sync did not settle");
            let mut progressed = false;
            if let Some(msg) = peer_a.generate_sync_message(&mut state_a) {
                peer_b.receive_sync_message(&mut state_b, &msg).unwrap();
                progressed = true;
            }
            if let Some(msg) = peer_b.generate_sync_message(&mut state_b) {
                peer_a.receive_sync_message(&mut state_a, &msg).unwrap();
                progressed = true;
            }
            if !progressed {
                break;
            }
        }

        let mut a_uuids = peer_a.task_uuids().unwrap();
        let mut b_uuids = peer_b.task_uuids().unwrap();
        a_uuids.sort();
        b_uuids.sort();
        assert_eq!(
            a_uuids,
            vec![
                "00000000-0000-4000-8000-000000000002",
                "00000000-0000-4000-8000-000000000003"
            ]
        );
        assert_eq!(a_uuids, b_uuids);
    }
}
