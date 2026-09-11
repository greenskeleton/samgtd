//! Runnable synthetic acceptance demo for Milestone 001.
//!
//! Launches two (briefly, three) real `samgtdd` binaries against temporary
//! SQLite stores and real loopback TCP sockets — no external services, and
//! never touches a real/user database (`SAMGTD_DB_PATH` always points into a
//! throwaway temp directory this demo creates). Prints a transcript of every
//! assertion as it runs, writes a machine-readable results JSON and a
//! readable transcript file under `artifacts/acceptance/`, and exits
//! nonzero if any check failed.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p samgtdd --example demo
//! ```
//!
//! Shares its scenario implementation with the automated test at
//! `crates/samgtdd/tests/two_process_acceptance.rs` via `samgtd-testkit`.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // crates/samgtdd/examples/demo.rs -> CARGO_MANIFEST_DIR is crates/samgtdd;
    // the repo root is two levels up, per this workspace's fixed layout
    // (see README.md, "Repo layout target").
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest_dir)
}

/// `CARGO_BIN_EXE_<name>` (used by the integration test) is only set for
/// tests/benches, not examples, so an example can't rely on Cargo having
/// already built `samgtdd`'s `[[bin]]` as a side effect of building this
/// example. Build it explicitly (a no-op if already up to date) and return
/// its deterministic `target/debug` path, unless `SAMGTD_DAEMON_BIN` names
/// an already-built binary to use instead.
async fn ensure_daemon_binary(repo_root: &Path) -> PathBuf {
    if let Ok(explicit) = std::env::var("SAMGTD_DAEMON_BIN") {
        return PathBuf::from(explicit);
    }
    let status = tokio::process::Command::new("cargo")
        .args(["build", "--quiet", "-p", "samgtdd", "--bin", "samgtdd"])
        .current_dir(repo_root)
        .status()
        .await
        .expect("invoke `cargo build -p samgtdd --bin samgtdd`");
    if !status.success() {
        eprintln!("`cargo build -p samgtdd --bin samgtdd` failed; cannot run the demo");
        std::process::exit(2);
    }
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("target"));
    let name = if cfg!(windows) {
        "samgtdd.exe"
    } else {
        "samgtdd"
    };
    target_dir.join("debug").join(name)
}

#[tokio::main]
async fn main() {
    let repo_root = repo_root();
    let bin = ensure_daemon_binary(&repo_root).await;

    let work_dir = std::env::temp_dir().join(format!("samgtd-demo-{}", std::process::id()));
    let out_dir = std::env::var("SAMGTD_DEMO_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("artifacts").join("acceptance"));

    println!("samgtd Milestone 001 synthetic acceptance demo");
    println!("daemon binary:      {}", bin.display());
    println!("synthetic stores:   {}", work_dir.display());
    println!("results artifacts:  {}", out_dir.display());
    println!();

    let provenance = match samgtd_testkit::provenance::collect(&repo_root).await {
        Ok(p) => p,
        Err(err) => {
            eprintln!("failed to collect toolchain/source provenance: {err:#}");
            std::process::exit(2);
        }
    };

    let report = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        samgtd_testkit::scenario::run_full(&bin, &work_dir),
    )
    .await
    .unwrap_or_else(|_| {
        eprintln!("acceptance scenario exceeded its overall 180s bound");
        std::process::exit(2);
    });

    println!("{}", report.transcript());

    match samgtd_testkit::artifact::write(
        &out_dir,
        "milestone-001-two-process",
        &provenance,
        &report,
    ) {
        Ok(written) => {
            println!("wrote {}", written.json_path.display());
            println!("wrote {}", written.transcript_path.display());
        }
        Err(err) => {
            eprintln!("failed to write results artifact: {err:#}");
            std::process::exit(2);
        }
    }

    let _ = std::fs::remove_dir_all(&work_dir);

    if report.all_passed() {
        println!("RESULT: PASS");
        std::process::exit(0);
    } else {
        println!("RESULT: FAIL");
        std::process::exit(1);
    }
}
