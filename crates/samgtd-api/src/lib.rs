#![forbid(unsafe_code)]

//! HTTP/WebSocket wire contracts.
//!
//! This crate holds request/response and message *shapes* shared between the
//! daemon and future clients (TUI/web/Android/voice/MCP). It must not depend
//! on Axum's server internals or on the domain/CRDT/store crates.
//!
pub mod health;
pub mod sync;
