//! Real two-process acceptance test: two actual `samgtdd` binaries, each
//! with its own temporary SQLite store, bound to real loopback TCP ports.
//! Exercises their HTTP and `/sync` WebSocket interfaces over real sockets
//! via `samgtd-testkit`, which also provides the relay used by
//! `examples/demo.rs` — the two share the same scenario implementation
//! (`samgtd_testkit::scenario::run_full`), so this test and the runnable
//! demo cannot silently drift apart.
//!
//! See `docs/protocol.md` and `docs/milestone-001-acceptance.md` for how
//! each of the milestone's required scenario steps maps to the checks this
//! test asserts on.

#[tokio::test(flavor = "multi_thread")]
async fn two_real_daemons_converge_persist_and_recover_over_real_tcp() {
    let bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_samgtdd"));
    let dir = tempfile::tempdir().expect("create scratch dir for synthetic peers/stores");

    let report = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        samgtd_testkit::scenario::run_full(&bin, dir.path()),
    )
    .await
    .expect("acceptance scenario exceeded its overall bound");

    let transcript = report.transcript();
    println!("{transcript}");

    if !report.all_passed() {
        if let Some(first) = report.first_failure() {
            panic!(
                "acceptance scenario failed at check {:?}: {}\n\nfull transcript:\n{transcript}",
                first.id, first.detail
            );
        }
        panic!("acceptance scenario recorded no checks at all\n\n{transcript}");
    }
}
