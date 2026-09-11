#![forbid(unsafe_code)]

//! HTTP/WebSocket wire contracts.
//!
//! This crate holds request/response and message *shapes* shared between the
//! daemon and future clients (TUI/web/Android/voice/MCP). It must not depend
//! on Axum's server internals or on the domain/CRDT/store crates.
//!
//! Only the `/health` contract is defined so far. GTD entity DTOs and the
//! Automerge sync WebSocket message framing are deferred until the existing
//! SQLite database has been inventoried (see `AGENTS.md`, "Existing database
//! compatibility") and the CRDT document-granularity ADR is accepted.

pub mod health;
