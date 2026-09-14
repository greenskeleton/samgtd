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

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::{fs::File, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("store is already owned or cannot be locked: {0}")]
    Lock(String),
    #[error("refusing an unrecognized database; use a new samgtd store path")]
    ForeignDatabase,
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// A samgtd-owned SQLite store for Automerge document bytes, keyed by
/// `(doc_type, doc_id)` — e.g. `("root_index", "default")` or
/// `("task", "<uuid>")`.
pub struct Store {
    conn: Connection,
    _lock: Option<File>,
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
        if path.exists() {
            let probe = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let marker: i64 = probe.pragma_query_value(None, "application_id", |r| r.get(0))?;
            if marker != 0x53475444 {
                return Err(StoreError::ForeignDatabase);
            }
        }
        let canonical = if path.exists() {
            path.canonicalize()
        } else {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            parent
                .canonicalize()
                .map(|p| p.join(path.file_name().unwrap_or_default()))
        }
        .map_err(|e| StoreError::Lock(e.to_string()))?;
        let mut lock_path = canonical.as_os_str().to_os_string();
        lock_path.push(".samgtd-lock");
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(Path::new(&lock_path))
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        lock.try_lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        if path.exists() {
            let probe = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let marker: i64 = probe.pragma_query_value(None, "application_id", |r| r.get(0))?;
            if marker != 0x53475444 {
                return Err(StoreError::ForeignDatabase);
            }
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "application_id", 0x53475444_i64)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn,
            _lock: Some(lock),
        })
    }

    /// Open an ephemeral in-memory store (tests only).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn, _lock: None })
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

    /// Commit related opaque documents together; rollback every write on failure.
    pub fn save_documents(&mut self, documents: &[(&str, &str, &[u8])]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for (kind, id, bytes) in documents {
            tx.execute(
                "INSERT INTO documents (doc_type, doc_id, bytes) VALUES (?1, ?2, ?3)
                 ON CONFLICT(doc_type, doc_id) DO UPDATE SET bytes = excluded.bytes",
                rusqlite::params![kind, id, bytes],
            )?;
        }
        tx.commit()?;
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
    fn batch_rolls_back_when_second_write_fails() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .save_document("root_index", "default", b"old")
            .unwrap();
        store
            .conn
            .execute_batch(
                "CREATE TRIGGER fail_task BEFORE INSERT ON documents
            WHEN NEW.doc_type = 'task' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
            )
            .unwrap();
        assert!(store
            .save_documents(&[("root_index", "default", b"new"), ("task", "t1", b"task"),])
            .is_err());
        assert_eq!(
            store.load_document("root_index", "default").unwrap(),
            Some(b"old".to_vec())
        );
        assert_eq!(store.load_document("task", "t1").unwrap(), None);
    }

    #[test]
    fn refuses_foreign_database_without_modifying_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE tasks(id INTEGER PRIMARY KEY, title TEXT);")
            .unwrap();
        drop(conn);
        let original = std::fs::read(&path).unwrap();
        assert!(matches!(
            Store::open(&path),
            Err(StoreError::ForeignDatabase)
        ));
        assert_eq!(original, std::fs::read(&path).unwrap());
        #[cfg(unix)]
        {
            let alias = dir.path().join("alias.sqlite");
            std::os::unix::fs::symlink(&path, &alias).unwrap();
            assert!(matches!(
                Store::open(&alias),
                Err(StoreError::ForeignDatabase)
            ));
            assert_eq!(original, std::fs::read(&path).unwrap());
        }
    }

    #[test]
    fn refuses_second_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.sqlite");
        let first = Store::open(&path).unwrap();
        assert!(matches!(Store::open(&path), Err(StoreError::Lock(_))));
        drop(first);
        assert!(Store::open(&path).is_ok());
    }

    #[test]
    fn missing_document_is_none() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.load_document("task", "missing").unwrap(), None);
    }
}
