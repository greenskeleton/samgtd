#![forbid(unsafe_code)]

//! Pure GTD domain types and rules.
//!
//! This crate must not depend on Axum or on SQLite directly (see
//! `AGENTS.md`, "Development rules").
//!
//! No entity types (Task/Project/Context/Category) are defined yet. The
//! existing SQLite database at `.local/current-gtd.sqlite` has not been
//! inventoried in this pass, and `AGENTS.md` ("Existing database
//! compatibility") requires that inventory before durable identity/schema
//! decisions are made here.
