//! Scenario/check result recording, shared by the two-process acceptance
//! test and the standalone demo binary (`crates/samgtdd/examples/demo.rs`).
//!
//! A [`Report`] is a flat, ordered list of [`Check`]s. Each check is either a
//! passing assertion (with a human-readable detail of what was observed) or
//! a failure (with the error that was returned). Nothing here decides
//! severity or aborts a scenario; callers decide whether to keep going after
//! a failed check (most milestone steps depend on earlier ones, so the
//! scenario function generally stops at the first failure and returns
//! whatever checks were recorded so far).

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub id: String,
    pub description: String,
    pub passed: bool,
    pub detail: String,
    pub at: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub checks: Vec<Check>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one check. `id` is a short stable machine-readable slug (e.g.
    /// `"2b-b-receives-task-via-sync"`); `description` is the human-readable
    /// milestone requirement it demonstrates. `Ok` carries what was actually
    /// observed; `Err` carries the failure message.
    pub fn record(&mut self, id: &str, description: &str, result: Result<String, String>) {
        let (passed, detail) = match result {
            Ok(detail) => (true, detail),
            Err(detail) => (false, detail),
        };
        self.checks.push(Check {
            id: id.to_string(),
            description: description.to_string(),
            passed,
            detail,
            at: iso8601_now(),
        });
    }

    pub fn all_passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|c| c.passed)
    }

    pub fn first_failure(&self) -> Option<&Check> {
        self.checks.iter().find(|c| !c.passed)
    }

    pub fn transcript(&self) -> String {
        let mut out = String::new();
        for check in &self.checks {
            let mark = if check.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!(
                "[{mark}] {} — {}\n    {}\n    at {}\n",
                check.id, check.description, check.detail, check.at
            ));
        }
        let total = self.checks.len();
        let passed = self.checks.iter().filter(|c| c.passed).count();
        out.push_str(&format!("\n{passed}/{total} checks passed\n"));
        out
    }
}

/// Format the current wall-clock time as an ISO-8601 / RFC-3339 UTC
/// timestamp without pulling in a date/time crate.
pub fn iso8601_now() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_unix_seconds(dur.as_secs(), dur.subsec_millis())
}

fn format_unix_seconds(total_secs: u64, millis: u32) -> String {
    // Civil-from-days conversion (Howard Hinnant's algorithm), good for any
    // date this project will ever run on; avoids a chrono/time dependency.
    let days = (total_secs / 86_400) as i64;
    let secs_of_day = total_secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_epoch_seconds_format_correctly() {
        // 2026-09-11T00:00:00Z per the day this project's scenario data uses.
        assert_eq!(
            format_unix_seconds(1_789_084_800, 0),
            "2026-09-11T00:00:00.000Z"
        );
        assert_eq!(format_unix_seconds(0, 0), "1970-01-01T00:00:00.000Z");
    }
}
