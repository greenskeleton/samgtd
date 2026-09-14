//! Application operations. Documents are loaded into candidates and committed
//! before returning, so failed writes cannot leak into served state — but
//! `Service` itself does not serialize concurrent callers; nothing about its
//! own type stops two threads from interleaving, say, a `read`-then-`write`
//! in `update()`. In the daemon, serialization is enforced one layer up, by
//! `transport::App::call` holding a `Mutex<Service>` around every operation.
//! A future direct embedder of this type (e.g. FFI) must provide the same
//! external serialization itself.
use anyhow::{bail, Context};
use automerge::sync;
use samgtd_crdt::{RootIndexDocument, TaskDocument, TaskFields};
use samgtd_store::Store;
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;

pub use samgtd_api::sync::{Envelope, Identity, Provision, TaskPatch};
pub type Session = BTreeMap<(String, String), sync::State>;

pub struct Service {
    store: Store,
    pub identity: Identity,
}
impl Service {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut store = Store::open(path)?;
        let identity = match store.load_document("local", "identity")? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => {
                let identity = Identity {
                    node: Uuid::new_v4().to_string(),
                    dataset: Uuid::new_v4().to_string(),
                };
                let root = match store.load_document("root_index", "default")? {
                    Some(bytes) => bytes,
                    None => RootIndexDocument::new()?.save(),
                };
                let bytes = serde_json::to_vec(&identity)?;
                store.save_documents(&[
                    ("local", "identity", &bytes),
                    ("root_index", "default", &root),
                ])?;
                identity
            }
        };
        Ok(Self { store, identity })
    }
    fn root(&self) -> anyhow::Result<RootIndexDocument> {
        Ok(RootIndexDocument::load(
            &self
                .store
                .load_document("root_index", "default")?
                .context("missing root")?,
        )?)
    }
    pub fn provision(&mut self) -> anyhow::Result<Provision> {
        Ok(Provision {
            dataset: self.identity.dataset.clone(),
            root: self.root()?.save(),
        })
    }
    /// Explicitly adopt a dataset only before this replica has any tasks,
    /// and only once ever. Guarding solely on "currently has zero tasks"
    /// would let a second, stray `POST /provision` silently reassign an
    /// already-joined-but-still-empty replica to a different dataset; a
    /// sticky `local/joined` marker (written only by this method) closes
    /// that gap regardless of task count.
    pub fn join(&mut self, provision: Provision) -> anyhow::Result<()> {
        Uuid::parse_str(&provision.dataset)?;
        if self.store.load_document("local", "joined")?.is_some() {
            bail!("replica has already explicitly joined a dataset");
        }
        if !self.root()?.task_uuids()?.is_empty()
            || !self.store.list_document_ids("task")?.is_empty()
        {
            bail!("joining requires an empty replica");
        }
        let root = RootIndexDocument::load(&provision.root)?;
        for id in root.task_uuids()? {
            Uuid::parse_str(&id)?;
        }
        let identity = Identity {
            node: self.identity.node.clone(),
            dataset: provision.dataset,
        };
        self.store.save_documents(&[
            ("local", "identity", &serde_json::to_vec(&identity)?),
            ("root_index", "default", &provision.root),
            ("local", "joined", &[1u8]),
        ])?;
        self.identity = identity;
        Ok(())
    }
    pub fn create(&mut self, mut fields: TaskFields) -> anyhow::Result<TaskFields> {
        fields.uuid = Uuid::new_v4().to_string();
        validate(&fields)?;
        let mut doc = TaskDocument::new(&fields)?;
        let mut root = self.root()?;
        root.add_task(&fields.uuid)?;
        self.store.save_documents(&[
            ("task", &fields.uuid, &doc.save()),
            ("root_index", "default", &root.save()),
        ])?;
        Ok(fields)
    }
    pub fn read(&self, id: &str) -> anyhow::Result<TaskFields> {
        Uuid::parse_str(id)?;
        let bytes = self
            .store
            .load_document("task", id)?
            .context("task unavailable")?;
        let fields = TaskDocument::load(&bytes)?.fields()?;
        validate(&fields)?;
        if fields.uuid != id {
            bail!("task identity mismatch");
        }
        Ok(fields)
    }
    pub fn update(&mut self, id: &str, patch: TaskPatch) -> anyhow::Result<TaskFields> {
        self.read(id)?;
        let mut doc = TaskDocument::load(
            &self
                .store
                .load_document("task", id)?
                .context("task unavailable")?,
        )?;
        if let Some(title) = patch.title {
            doc.set_title(&title)?;
        }
        if let Some(notes) = patch.notes {
            doc.set_notes(&notes)?;
        }
        if let Some(status) = patch.status {
            doc.set_status(&status)?;
        }
        let fields = doc.fields()?;
        validate(&fields)?;
        self.store.save_document("task", id, &doc.save())?;
        Ok(fields)
    }
    pub fn exchange(
        &mut self,
        states: &mut Session,
        incoming: Option<Envelope>,
    ) -> anyhow::Result<Vec<Envelope>> {
        if let Some(msg) = incoming {
            if msg.version != 1 || msg.dataset != self.identity.dataset {
                bail!("dataset or protocol mismatch");
            }
            let state = states
                .entry((msg.kind.clone(), msg.id.clone()))
                .or_default();
            match msg.kind.as_str() {
                "root_index" if msg.id == "default" => {
                    let mut root = self.root()?;
                    root.receive_sync_message(state, &msg.message)?;
                    for id in root.task_uuids()? {
                        Uuid::parse_str(&id)?;
                    }
                    self.store
                        .save_document("root_index", "default", &root.save())?;
                }
                "task" => {
                    Uuid::parse_str(&msg.id)?;
                    if !self.root()?.task_uuids()?.contains(&msg.id) {
                        bail!("unindexed task");
                    }
                    let mut doc = self.task_receiver(&msg.id)?;
                    doc.receive_sync_message(state, &msg.message)?;
                    // Empty receivers may remain incomplete during the handshake.
                    if !doc.heads().is_empty() {
                        let fields = doc.fields()?;
                        validate(&fields)?;
                        if fields.uuid != msg.id {
                            bail!("task identity mismatch");
                        }
                    }
                    self.store.save_document("task", &msg.id, &doc.save())?;
                }
                _ => bail!("unknown document"),
            }
        }
        let mut outgoing = Vec::new();
        let mut root = self.root()?;
        let ids = root.task_uuids()?;
        let state = states
            .entry(("root_index".into(), "default".into()))
            .or_default();
        if let Some(message) = root.generate_sync_message(state) {
            outgoing.push(self.envelope("root_index", "default", message));
        }
        // Do not send task frames before the remote has acknowledged membership.
        if state.shared_heads != root.heads() {
            return Ok(outgoing);
        }
        for id in ids {
            let mut doc = self.task_receiver(&id)?;
            let state = states.entry(("task".into(), id.clone())).or_default();
            if let Some(message) = doc.generate_sync_message(state) {
                outgoing.push(self.envelope("task", &id, message));
            }
        }
        Ok(outgoing)
    }
    fn task_receiver(&self, id: &str) -> anyhow::Result<TaskDocument> {
        Ok(match self.store.load_document("task", id)? {
            Some(bytes) => TaskDocument::load(&bytes)?,
            None => TaskDocument::empty(),
        })
    }
    fn envelope(&self, kind: &str, id: &str, message: Vec<u8>) -> Envelope {
        Envelope {
            version: 1,
            dataset: self.identity.dataset.clone(),
            kind: kind.into(),
            id: id.into(),
            message,
        }
    }
}
fn validate(fields: &TaskFields) -> anyhow::Result<()> {
    Ok(fields.validate()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_is_rejected_after_the_replica_has_already_joined_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut origin_x = Service::open(&dir.path().join("x.sqlite")).unwrap();
        let mut origin_y = Service::open(&dir.path().join("y.sqlite")).unwrap();
        let provision_x = origin_x.provision().unwrap();
        let provision_y = origin_y.provision().unwrap();

        let mut joiner = Service::open(&dir.path().join("joiner.sqlite")).unwrap();
        joiner.join(provision_x).unwrap();
        let dataset_after_first_join = joiner.identity.dataset.clone();

        // Still empty, but a second join (e.g. a stray retried request)
        // must not silently reassign the replica to a different dataset.
        assert!(joiner.join(provision_y).is_err());
        assert_eq!(joiner.identity.dataset, dataset_after_first_join);
    }
}
