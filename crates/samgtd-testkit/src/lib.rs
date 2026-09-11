#![forbid(unsafe_code)]

//! Test/demo-only harness for exercising real `samgtdd` binaries over real
//! loopback TCP sockets: process spawn/lifecycle, a minimal HTTP client, a
//! transparent `/sync` WebSocket relay, post-mortem SQLite document
//! inspection, and acceptance-run reporting/provenance.
//!
//! This crate is never a production dependency of `samgtdd` itself — it is
//! only ever a `[dev-dependencies]` of the daemon crate, reached from its
//! `tests/` and `examples/` targets. See `crates/samgtdd/tests/two_process_acceptance.rs`
//! and `crates/samgtdd/examples/demo.rs`.

pub mod artifact;
pub mod http;
pub mod process;
pub mod provenance;
pub mod relay;
pub mod report;
pub mod scenario;
pub mod store;

pub use process::Daemon;
pub use relay::Relay;
pub use report::Report;
