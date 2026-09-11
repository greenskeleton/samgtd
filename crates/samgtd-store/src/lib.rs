#![forbid(unsafe_code)]

//! SQLite persistence for Automerge document bytes.
//!
//! This crate does **not** open or touch `.local/current-gtd.sqlite`. Per
//! `docs/adr/0003-existing-database-coexistence.md`, that database's schema
//! is owned by an existing external application and this project has not
//! yet resolved the live-coexistence question. `Store` manages a separate,
//! samgtd-owned database file containing only Automerge document blobs; it
//! has no GTD table schema of its own to conflict with anything.
//!
//! Automerge state is authoritative for replicated GTD entities (see
//! `README.md`, "Persistence") — this crate only stores/retrieves the
//! opaque bytes `samgtd-crdt` produces, and does not interpret them.

use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// A samgtd-owned SQLite store for Automerge document bytes, keyed by
/// `(doc_type, doc_id)` — e.g. `("root_index", "default")` or
/// `("task", "<uuid>")`.
pub struct Store {
    conn: Connection,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS documents (
        doc_type TEXT NOT NULL,
        doc_id   TEXT NOT NULL,
        bytes    BLOB NOT NULL,
        PRIMARY KEY (doc_type, doc_id)
    );
";

impl Store {
    /// Open (creating if needed) a store backed by a file on disk.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Open an ephemeral in-memory store (tests only).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn save_document(
        &self,
        doc_type: &str,
        doc_id: &str,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO documents (doc_type, doc_id, bytes) VALUES (?1, ?2, ?3)
             ON CONFLICT(doc_type, doc_id) DO UPDATE SET bytes = excluded.bytes",
            rusqlite::params![doc_type, doc_id, bytes],
        )?;
        Ok(())
    }

    pub fn load_document(
        &self,
        doc_type: &str,
        doc_id: &str,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.conn
            .query_row(
                "SELECT bytes FROM documents WHERE doc_type = ?1 AND doc_id = ?2",
                rusqlite::params![doc_type, doc_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn list_document_ids(&self, doc_type: &str) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT doc_id FROM documents WHERE doc_type = ?1")?;
        let rows = stmt.query_map([doc_type], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_document_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.sqlite");

        {
            let store = Store::open(&path).unwrap();
            store.save_document("task", "t1", b"hello").unwrap();
        }

        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.load_document("task", "t1").unwrap(),
            Some(b"hello".to_vec())
        );
        assert_eq!(store.list_document_ids("task").unwrap(), vec!["t1"]);
    }

    #[test]
    fn missing_document_is_none() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.load_document("task", "missing").unwrap(), None);
    }
}
