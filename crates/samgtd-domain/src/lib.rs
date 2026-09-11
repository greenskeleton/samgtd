#![forbid(unsafe_code)]
//! Pure domain values and validation, independent of persistence and transport.
use thiserror::Error;
use uuid::Uuid;

/// Fields for a single Task, mirroring `docs/existing-database.md`'s `tasks`
/// table. `category_id`/`project_id`/`domain_id` are `samgtd_identity` UUIDs
/// (see `docs/adr/0003-existing-database-coexistence.md`), not the
/// underlying SQLite integer ids.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskFields {
    #[serde(default)]
    pub uuid: String,
    pub title: String,
    pub notes: String,
    /// `"TODO"` or `"DONE"`, matching the existing database's stored values.
    pub status: String,
    pub category_id: String,
    pub project_id: Option<String>,
    pub domain_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("invalid UUID: {0}")]
    Identity(#[from] uuid::Error),
    #[error("status must be TODO or DONE")]
    Status,
}

/// The single place task status validity is decided. `samgtd-crdt`'s
/// `TaskDocument::set_status` calls this too, so the two crates cannot drift
/// out of sync the way they previously did (a status `set_status` accepted
/// but `TaskFields::validate` then rejected on next read).
pub fn is_valid_status(status: &str) -> bool {
    matches!(status, "TODO" | "DONE")
}

impl TaskFields {
    pub fn validate(&self) -> Result<(), TaskError> {
        Uuid::parse_str(&self.uuid)?;
        Uuid::parse_str(&self.category_id)?;
        for id in [&self.project_id, &self.domain_id].into_iter().flatten() {
            Uuid::parse_str(id)?;
        }
        if !is_valid_status(&self.status) {
            return Err(TaskError::Status);
        }
        Ok(())
    }
}
