//! Toolchain and source-revision provenance for the acceptance results
//! artifact.
//!
//! A commit ID alone does not describe what actually ran when the working
//! tree is dirty (which is the normal state for an in-progress milestone).
//! When dirty, this also hashes the modified/untracked files `git status`
//! reports, so the results artifact records what was really on disk, not
//! just the last commit.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Toolchain {
    pub rustc: String,
    pub cargo: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub commit: Option<String>,
    pub dirty: bool,
    /// `sha256:<hex>` over sorted `(status, path, contents)` triples from
    /// `git status --porcelain=v1 -uall --no-renames`. Present only when
    /// `dirty` is true.
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub toolchain: Toolchain,
    pub source: Source,
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn run(cmd: &str, args: &[&str], cwd: &Path) -> anyhow::Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;
    anyhow::ensure!(
        output.status.success(),
        "{cmd} {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub async fn collect(repo_root: &Path) -> anyhow::Result<Provenance> {
    let rustc = run("rustc", &["--version"], repo_root)
        .await
        .unwrap_or_else(|e| format!("unavailable: {e}"));
    let cargo = run("cargo", &["--version"], repo_root)
        .await
        .unwrap_or_else(|e| format!("unavailable: {e}"));

    let commit = run("git", &["rev-parse", "HEAD"], repo_root).await.ok();
    let status = run(
        "git",
        &["status", "--porcelain=v1", "-uall", "--no-renames"],
        repo_root,
    )
    .await
    .unwrap_or_default();

    let mut entries: Vec<(String, String)> = status
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let code = line[..2].to_string();
            let path = line[3..].to_string();
            Some((code, path))
        })
        .collect();
    entries.sort();

    let dirty = !entries.is_empty();
    let fingerprint = if dirty {
        let mut hasher = Sha256::new();
        for (code, path) in &entries {
            hasher.update(code.as_bytes());
            hasher.update(b" ");
            hasher.update(path.as_bytes());
            hasher.update(b"\n");
            let full = repo_root.join(path);
            match std::fs::read(&full) {
                Ok(bytes) => {
                    hasher.update(b"present:");
                    hasher.update(bytes);
                }
                Err(_) => {
                    hasher.update(b"absent");
                }
            }
            hasher.update(b"\n---\n");
        }
        Some(format!("sha256:{}", to_hex(&hasher.finalize())))
    } else {
        None
    };

    Ok(Provenance {
        toolchain: Toolchain { rustc, cargo },
        source: Source {
            commit,
            dirty,
            fingerprint,
        },
    })
}
