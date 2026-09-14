//! Writes the machine-readable results artifact and the human-readable
//! transcript described in the milestone's acceptance-evidence requirements.
//! Shared by the integration test (which writes into a temp dir) and the
//! demo binary (which writes into a committed-but-ignored artifacts dir).

use crate::{provenance::Provenance, report::Report};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ResultsArtifact<'a> {
    schema_version: u32,
    generated_at: String,
    scenario: &'a str,
    result: &'a str,
    toolchain: &'a crate::provenance::Toolchain,
    source: &'a crate::provenance::Source,
    checks: &'a [crate::report::Check],
}

pub struct Written {
    pub json_path: PathBuf,
    pub transcript_path: PathBuf,
}

fn filename_timestamp() -> String {
    crate::report::iso8601_now().replace([':', '.'], "-")
}

/// Write both a timestamped copy and a stable `-latest` copy of each
/// artifact, so CI can upload everything while a human/script can always
/// find the most recent run without parsing filenames.
pub fn write(
    out_dir: &Path,
    scenario: &str,
    provenance: &Provenance,
    report: &Report,
) -> anyhow::Result<Written> {
    std::fs::create_dir_all(out_dir)?;
    let result = if report.all_passed() { "pass" } else { "fail" };
    let artifact = ResultsArtifact {
        schema_version: 1,
        generated_at: crate::report::iso8601_now(),
        scenario,
        result,
        toolchain: &provenance.toolchain,
        source: &provenance.source,
        checks: &report.checks,
    };
    let json = serde_json::to_string_pretty(&artifact)?;
    let transcript = render_transcript(&artifact, report);

    let stamp = filename_timestamp();
    let json_path = out_dir.join(format!("results-{stamp}.json"));
    let transcript_path = out_dir.join(format!("transcript-{stamp}.txt"));
    std::fs::write(&json_path, &json)?;
    std::fs::write(&transcript_path, &transcript)?;
    std::fs::write(out_dir.join("results-latest.json"), &json)?;
    std::fs::write(out_dir.join("transcript-latest.txt"), &transcript)?;

    Ok(Written {
        json_path,
        transcript_path,
    })
}

fn render_transcript(artifact: &ResultsArtifact<'_>, report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "samgtd Milestone 001 acceptance — {}\n",
        artifact.scenario
    ));
    out.push_str(&format!("generated_at: {}\n", artifact.generated_at));
    out.push_str(&format!(
        "toolchain: {} / {}\n",
        artifact.toolchain.rustc, artifact.toolchain.cargo
    ));
    match &artifact.source.commit {
        Some(commit) => out.push_str(&format!("source commit: {commit}\n")),
        None => out.push_str("source commit: <unavailable>\n"),
    }
    out.push_str(&format!("working tree dirty: {}\n", artifact.source.dirty));
    if let Some(fingerprint) = &artifact.source.fingerprint {
        out.push_str(&format!("source fingerprint: {fingerprint}\n"));
    }
    out.push_str(&format!("result: {}\n\n", artifact.result));
    out.push_str(&report.transcript());
    out
}
