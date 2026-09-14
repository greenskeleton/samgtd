//! The Milestone 001 two-process acceptance scenario.
//!
//! Everything here drives two (and, briefly, a third) real `samgtdd`
//! binaries over real loopback TCP sockets: HTTP requests go over actual
//! `TcpStream`s (`crate::http`), sync goes over real WebSocket connections
//! relayed between the two daemons' `/sync` endpoints (`crate::relay`), and
//! process lifecycle (start, ready, graceful/abrupt stop, restart) goes
//! through real child processes (`crate::process`). Document-head and
//! membership assertions that the public API doesn't expose are answered by
//! reading each store only after its daemon has fully exited
//! (`crate::store`). No step reads or mutates one replica's database to
//! drive another replica's synchronization — sync only ever happens through
//! the relay.
//!
//! Shared by `crates/samgtdd/tests/two_process_acceptance.rs` and
//! `crates/samgtdd/examples/demo.rs`.

use crate::{http, process::Daemon, relay::Relay, report::Report, store};
use anyhow::Context;
use serde_json::{json, Value};
use std::{net::SocketAddr, path::Path, time::Duration};
use tokio::time::Instant;

const STARTUP_BOUND: Duration = Duration::from_secs(10);
const SYNC_BOUND: Duration = Duration::from_secs(15);
const SHUTDOWN_BOUND: Duration = Duration::from_secs(15);
const CATEGORY: &str = "00000000-0000-4000-8000-000000000001";

async fn step<T>(
    report: &mut Report,
    id: &str,
    description: &str,
    fut: impl std::future::Future<Output = anyhow::Result<(T, String)>>,
) -> anyhow::Result<T> {
    match fut.await {
        Ok((value, detail)) => {
            report.record(id, description, Ok(detail));
            Ok(value)
        }
        Err(err) => {
            report.record(id, description, Err(format!("{err:#}")));
            Err(err)
        }
    }
}

fn new_task(title: &str) -> Value {
    json!({
        "uuid": "",
        "title": title,
        "notes": "",
        "status": "TODO",
        "category_id": CATEGORY,
        "project_id": Value::Null,
        "domain_id": Value::Null,
    })
}

async fn wait_converge_equal(
    addr_a: SocketAddr,
    addr_b: SocketAddr,
    path: &str,
    bound: Duration,
) -> anyhow::Result<Value> {
    let start = Instant::now();
    loop {
        let ra = http::get(addr_a, path, Duration::from_secs(2)).await;
        let rb = http::get(addr_b, path, Duration::from_secs(2)).await;
        if let (Ok(ra), Ok(rb)) = (ra, rb) {
            if ra.status == 200 && rb.status == 200 && ra.body == rb.body {
                return Ok(ra.body);
            }
        }
        if start.elapsed() > bound {
            anyhow::bail!(
                "{path} did not converge to an equal value on both peers within {bound:?}"
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Steps 1-7 of the milestone scenario: provisioning, offline task creation,
/// independent-field convergence, a same-field conflict, idle-connection
/// propagation, and restart-with-full-state-and-heads verification.
pub async fn run_two_process_scenario(
    bin: &Path,
    out_dir: &Path,
    report: &mut Report,
) -> anyhow::Result<()> {
    step(
        report,
        "0-workdir",
        "create isolated scenario working directory",
        async {
            std::fs::create_dir_all(out_dir).context("create scenario output dir")?;
            Ok((
                (),
                format!("using isolated working directory {}", out_dir.display()),
            ))
        },
    )
    .await?;
    let dir_a = out_dir.join("peer-a");
    let dir_b = out_dir.join("peer-b");

    // --- Step 1: start A and B, readiness, health, identity, provisioning ---
    let mut daemon_a = step(report, "1a-spawn-a", "spawn peer A process", async {
        let d = Daemon::spawn(bin, &dir_a, "A")?;
        Ok((d, "spawned".to_string()))
    })
    .await?;
    let mut daemon_b = step(report, "1a-spawn-b", "spawn peer B process", async {
        let d = Daemon::spawn(bin, &dir_b, "B")?;
        Ok((d, "spawned".to_string()))
    })
    .await?;

    let addr_a = step(
        report,
        "1a-ready-a",
        "peer A becomes ready on a real loopback TCP listener (bounded polling, no fixed sleep)",
        async {
            let addr = daemon_a.wait_ready(STARTUP_BOUND).await?;
            Ok((addr, format!("bound {addr}")))
        },
    )
    .await?;
    let addr_b = step(
        report,
        "1a-ready-b",
        "peer B becomes ready on a real loopback TCP listener",
        async {
            let addr = daemon_b.wait_ready(STARTUP_BOUND).await?;
            Ok((addr, format!("bound {addr}")))
        },
    )
    .await?;

    step(
        report,
        "1a-loopback",
        "both listeners are loopback addresses",
        async {
            anyhow::ensure!(addr_a.ip().is_loopback(), "A bound {addr_a}, not loopback");
            anyhow::ensure!(addr_b.ip().is_loopback(), "B bound {addr_b}, not loopback");
            Ok(((), format!("A={addr_a} B={addr_b}")))
        },
    )
    .await?;

    step(
        report,
        "1a-health",
        "both peers report healthy over real HTTP",
        async {
            http::wait_health(addr_a, STARTUP_BOUND)
                .await
                .with_context(|| {
                    format!(
                        "A health; A captured output:\n{}",
                        daemon_a.captured_output().render()
                    )
                })?;
            http::wait_health(addr_b, STARTUP_BOUND)
                .await
                .with_context(|| {
                    format!(
                        "B health; B captured output:\n{}",
                        daemon_b.captured_output().render()
                    )
                })?;
            Ok(((), "GET /health returned 200 on both peers".to_string()))
        },
    )
    .await?;

    let identity_a = step(
        report,
        "1a-identity-a",
        "read peer A identity over HTTP",
        async {
            let resp = http::get(addr_a, "/identity", Duration::from_secs(5)).await?;
            anyhow::ensure!(
                resp.status == 200,
                "GET /identity on A returned {}",
                resp.status
            );
            Ok((resp.body.clone(), resp.body.to_string()))
        },
    )
    .await?;
    let identity_b = step(
        report,
        "1a-identity-b",
        "read peer B identity over HTTP",
        async {
            let resp = http::get(addr_b, "/identity", Duration::from_secs(5)).await?;
            anyhow::ensure!(
                resp.status == 200,
                "GET /identity on B returned {}",
                resp.status
            );
            Ok((resp.body.clone(), resp.body.to_string()))
        },
    )
    .await?;
    step(
        report,
        "1a-distinct-nodes",
        "peers have distinct node UUIDs before provisioning",
        async {
            anyhow::ensure!(
                identity_a["node"] != identity_b["node"],
                "A and B minted the same node UUID: {}",
                identity_a["node"]
            );
            Ok((
                (),
                format!(
                    "A.node={} B.node={}",
                    identity_a["node"], identity_b["node"]
                ),
            ))
        },
    )
    .await?;

    step(
        report,
        "1b-provision",
        "provision empty B into A's dataset via real HTTP GET/POST /provision",
        async {
            let bootstrap = http::get(addr_a, "/provision", Duration::from_secs(5)).await?;
            anyhow::ensure!(
                bootstrap.status == 200,
                "GET /provision on A returned {}",
                bootstrap.status
            );
            let joined = http::post(
                addr_b,
                "/provision",
                &bootstrap.body,
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(
                joined.status == 204,
                "POST /provision on B returned {}",
                joined.status
            );
            let identity_b2 = http::get(addr_b, "/identity", Duration::from_secs(5)).await?;
            anyhow::ensure!(
                identity_b2.body["dataset"] == identity_a["dataset"],
                "B's dataset after provisioning ({}) does not match A's ({})",
                identity_b2.body["dataset"],
                identity_a["dataset"]
            );
            anyhow::ensure!(
                identity_b2.body["node"] == identity_b["node"],
                "B's node identity changed across provisioning"
            );
            Ok((
                (),
                format!(
                    "B joined dataset {} while keeping node {}",
                    identity_b2.body["dataset"], identity_b2.body["node"]
                ),
            ))
        },
    )
    .await?;

    // --- Step 2: create a task on A while disconnected; connect; B discovers it ---
    let task1 = step(
        report,
        "2a-create-offline",
        "create a task on A while disconnected from B",
        async {
            let resp = http::post(
                addr_a,
                "/tasks",
                &new_task("Offline task created on A"),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(
                resp.status == 201,
                "POST /tasks on A returned {}",
                resp.status
            );
            Ok((resp.body.clone(), format!("created {}", resp.body["uuid"])))
        },
    )
    .await?;
    let task1_id = task1["uuid"]
        .as_str()
        .context("created task missing uuid")?
        .to_string();

    let relay1 = step(
        report,
        "2b-connect-relay",
        "connect a real WebSocket relay between A's and B's /sync endpoints",
        async {
            let r = Relay::connect(addr_a, addr_b).await?;
            Ok((
                r,
                "relay dialed both /sync endpoints over real TCP".to_string(),
            ))
        },
    )
    .await?;
    step(
        report,
        "2c-b-receives-task",
        "B discovers and receives the offline-created task through native Automerge sync",
        async {
            let seen =
                http::wait_for_value(addr_b, &format!("/tasks/{task1_id}"), &task1, SYNC_BOUND)
                    .await?;
            Ok(((), format!("B's projection converged to: {seen}")))
        },
    )
    .await?;

    // --- Step 3: disconnect; independent-field offline edits + new offline tasks ---
    step(report, "3a-disconnect", "disconnect the sync link", async {
        relay1.disconnect().await?;
        Ok((
            (),
            "relay stopped forwarding and closed both sockets".to_string(),
        ))
    })
    .await?;

    let notes_edit = step(
        report,
        "3b-edit-a-notes",
        "edit the shared task's notes field on A while offline",
        async {
            let resp = http::patch(
                addr_a,
                &format!("/tasks/{task1_id}"),
                &json!({"notes": "Edited on A while offline"}),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 200, "PATCH on A returned {}", resp.status);
            Ok((resp.body.clone(), resp.body.to_string()))
        },
    )
    .await?;
    let title_edit = step(
        report,
        "3c-edit-b-title",
        "edit the shared task's title field on B while offline (different field than A)",
        async {
            let resp = http::patch(
                addr_b,
                &format!("/tasks/{task1_id}"),
                &json!({"title": "Edited on B while offline"}),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 200, "PATCH on B returned {}", resp.status);
            Ok((resp.body.clone(), resp.body.to_string()))
        },
    )
    .await?;
    let task2 = step(
        report,
        "3d-create-a-offline",
        "create an additional task on A while offline",
        async {
            let resp = http::post(
                addr_a,
                "/tasks",
                &new_task("Second task, created offline on A"),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(
                resp.status == 201,
                "POST /tasks on A returned {}",
                resp.status
            );
            Ok((resp.body.clone(), format!("created {}", resp.body["uuid"])))
        },
    )
    .await?;
    let task3 = step(
        report,
        "3e-create-b-offline",
        "create an additional task on B while offline",
        async {
            let resp = http::post(
                addr_b,
                "/tasks",
                &new_task("Third task, created offline on B"),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(
                resp.status == 201,
                "POST /tasks on B returned {}",
                resp.status
            );
            Ok((resp.body.clone(), format!("created {}", resp.body["uuid"])))
        },
    )
    .await?;
    let task2_id = task2["uuid"]
        .as_str()
        .context("task2 missing uuid")?
        .to_string();
    let task3_id = task3["uuid"]
        .as_str()
        .context("task3 missing uuid")?
        .to_string();

    // Expected merged task1: both independent field edits preserved together.
    let mut expected_task1 = task1.clone();
    expected_task1["notes"] = notes_edit["notes"].clone();
    expected_task1["title"] = title_edit["title"].clone();

    // --- Step 4: reconnect; assert preservation, all IDs, full values, equal heads ---
    let relay2 = step(report, "4a-reconnect", "reconnect the sync relay", async {
        let r = Relay::connect(addr_a, addr_b).await?;
        Ok((
            r,
            "relay reconnected over fresh real TCP WebSocket connections".to_string(),
        ))
    })
    .await?;
    step(
        report,
        "4b-both-edits-preserved-a",
        "A converges with BOTH independent offline field edits",
        async {
            let seen = http::wait_for_value(
                addr_a,
                &format!("/tasks/{task1_id}"),
                &expected_task1,
                SYNC_BOUND,
            )
            .await?;
            Ok(((), seen.to_string()))
        },
    )
    .await?;
    step(
        report,
        "4b-both-edits-preserved-b",
        "B converges with BOTH independent offline field edits",
        async {
            let seen = http::wait_for_value(
                addr_b,
                &format!("/tasks/{task1_id}"),
                &expected_task1,
                SYNC_BOUND,
            )
            .await?;
            Ok(((), seen.to_string()))
        },
    )
    .await?;
    step(
        report,
        "4c-task2-reaches-b",
        "the task created offline on A reaches B with complete projected fields",
        async {
            let seen =
                http::wait_for_value(addr_b, &format!("/tasks/{task2_id}"), &task2, SYNC_BOUND)
                    .await?;
            Ok(((), seen.to_string()))
        },
    )
    .await?;
    step(
        report,
        "4c-task3-reaches-a",
        "the task created offline on B reaches A with complete projected fields",
        async {
            let seen =
                http::wait_for_value(addr_a, &format!("/tasks/{task3_id}"), &task3, SYNC_BOUND)
                    .await?;
            Ok(((), seen.to_string()))
        },
    )
    .await?;
    step(
        report,
        "4d-relay-disconnect",
        "disconnect the relay before stopping peers for store inspection",
        async {
            relay2.disconnect().await?;
            Ok(((), "disconnected".to_string()))
        },
    )
    .await?;

    let elapsed_a = step(
        report,
        "4e-stop-a",
        "gracefully stop A for post-mortem store inspection",
        async {
            let e = daemon_a.terminate_gracefully(SHUTDOWN_BOUND).await?;
            Ok((e, format!("exited cleanly in {e:?}")))
        },
    )
    .await?;
    let elapsed_b = step(
        report,
        "4e-stop-b",
        "gracefully stop B for post-mortem store inspection",
        async {
            let e = daemon_b.terminate_gracefully(SHUTDOWN_BOUND).await?;
            Ok((e, format!("exited cleanly in {e:?}")))
        },
    )
    .await?;
    let _ = (elapsed_a, elapsed_b);

    let db_path_a = daemon_a.db_path.clone();
    let db_path_b = daemon_b.db_path.clone();
    step(
        report,
        "4f-heads-equal",
        "root and per-task Automerge heads are identical on both replicas after convergence",
        async {
            let summary_a = store::inspect(&db_path_a)?;
            let summary_b = store::inspect(&db_path_b)?;
            anyhow::ensure!(
                summary_a.task_uuids == summary_b.task_uuids,
                "task id sets differ: A={:?} B={:?}",
                summary_a.task_uuids,
                summary_b.task_uuids
            );
            let mut expected_ids = vec![task1_id.clone(), task2_id.clone(), task3_id.clone()];
            expected_ids.sort();
            anyhow::ensure!(
                summary_a.task_uuids == expected_ids,
                "unexpected task id set: {:?}",
                summary_a.task_uuids
            );
            anyhow::ensure!(
                summary_a.root_heads == summary_b.root_heads,
                "root index heads differ: A={:?} B={:?}",
                summary_a.root_heads,
                summary_b.root_heads
            );
            for id in &expected_ids {
                anyhow::ensure!(
                    summary_a.task_heads[id] == summary_b.task_heads[id],
                    "task {id} heads differ: A={:?} B={:?}",
                    summary_a.task_heads[id],
                    summary_b.task_heads[id]
                );
                anyhow::ensure!(
                    summary_a.task_fields[id] == summary_b.task_fields[id],
                    "task {id} fields differ between replicas"
                );
            }
            Ok((
                (),
                format!(
                    "task ids {expected_ids:?}; root heads {:?}",
                    summary_a.root_heads
                ),
            ))
        },
    )
    .await?;

    // --- Restart both from durable state before the same-field conflict step ---
    let mut daemon_a = step(
        report,
        "4g-restart-a",
        "restart A from its own durable store",
        async {
            let d = Daemon::restart(bin, &dir_a, &db_path_a, "A")?;
            Ok((d, "restarted".to_string()))
        },
    )
    .await?;
    let mut daemon_b = step(
        report,
        "4g-restart-b",
        "restart B from its own durable store",
        async {
            let d = Daemon::restart(bin, &dir_b, &db_path_b, "B")?;
            Ok((d, "restarted".to_string()))
        },
    )
    .await?;
    let addr_a = step(report, "4g-ready-a", "restarted A becomes ready", async {
        let a = daemon_a.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    let addr_b = step(report, "4g-ready-b", "restarted B becomes ready", async {
        let a = daemon_b.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    step(
        report,
        "4h-identity-persisted",
        "node/dataset identity is unchanged after restart",
        async {
            let ia = http::get(addr_a, "/identity", Duration::from_secs(5)).await?;
            let ib = http::get(addr_b, "/identity", Duration::from_secs(5)).await?;
            anyhow::ensure!(
                ia.body == identity_a,
                "A's identity changed across restart: {} vs {}",
                ia.body,
                identity_a
            );
            anyhow::ensure!(
                ib.body["node"] == identity_b["node"],
                "B's node identity changed across restart"
            );
            anyhow::ensure!(
                ib.body["dataset"] == identity_a["dataset"],
                "B's dataset changed across restart"
            );
            Ok(((), format!("A={} B={}", ia.body, ib.body)))
        },
    )
    .await?;
    step(
        report,
        "4i-state-persisted",
        "all three tasks and both preserved field edits survive restart",
        async {
            for (addr, name) in [(addr_a, "A"), (addr_b, "B")] {
                let t1 =
                    http::get(addr, &format!("/tasks/{task1_id}"), Duration::from_secs(5)).await?;
                anyhow::ensure!(
                    t1.body == expected_task1,
                    "{name} lost task1's converged state across restart: {}",
                    t1.body
                );
                let t2 =
                    http::get(addr, &format!("/tasks/{task2_id}"), Duration::from_secs(5)).await?;
                anyhow::ensure!(t2.body == task2, "{name} lost task2 across restart");
                let t3 =
                    http::get(addr, &format!("/tasks/{task3_id}"), Duration::from_secs(5)).await?;
                anyhow::ensure!(t3.body == task3, "{name} lost task3 across restart");
            }
            Ok((
                (),
                "all three tasks present with expected fields on both restarted peers".to_string(),
            ))
        },
    )
    .await?;

    // --- Step 5: same-field concurrent conflict, offline, then reconnect ---
    let value_a = "Conflicting notes written on A";
    let value_b = "Conflicting notes written on B";
    step(
        report,
        "5a-conflict-edit-a",
        "edit task1.notes on A while offline (conflict candidate A)",
        async {
            let resp = http::patch(
                addr_a,
                &format!("/tasks/{task1_id}"),
                &json!({"notes": value_a}),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 200, "PATCH on A returned {}", resp.status);
            Ok(((), resp.body.to_string()))
        },
    )
    .await?;
    step(
        report,
        "5b-conflict-edit-b",
        "edit task1.notes on B while offline, to a DIFFERENT value (conflict candidate B)",
        async {
            let resp = http::patch(
                addr_b,
                &format!("/tasks/{task1_id}"),
                &json!({"notes": value_b}),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 200, "PATCH on B returned {}", resp.status);
            Ok(((), resp.body.to_string()))
        },
    )
    .await?;
    let relay3 = step(
        report,
        "5c-reconnect",
        "reconnect the sync relay to resolve the conflict",
        async {
            let r = Relay::connect(addr_a, addr_b).await?;
            Ok((r, "reconnected".to_string()))
        },
    )
    .await?;
    let resolved = step(
        report,
        "5d-conflict-resolved-equally",
        "both peers converge to the SAME notes value, without this test assuming which one wins",
        async {
            let value =
                wait_converge_equal(addr_a, addr_b, &format!("/tasks/{task1_id}"), SYNC_BOUND)
                    .await?;
            let notes = value["notes"].as_str().unwrap_or_default().to_string();
            anyhow::ensure!(
                notes == value_a || notes == value_b,
                "converged notes value {notes:?} is neither concurrently-written candidate"
            );
            let detail = format!("both peers converged to notes={notes:?}");
            Ok((value, detail))
        },
    )
    .await?;

    // --- Step 6: a local edit propagates over the already-open, now-idle connection ---
    let mut idle_expected = resolved.clone();
    idle_expected["title"] = json!("Edited after reconnect, over an idle connection");
    step(
        report,
        "6a-idle-edit",
        "edit task1.title on A over the connection that is now idle (no disconnect/reconnect)",
        async {
            let resp = http::patch(
                addr_a,
                &format!("/tasks/{task1_id}"),
                &json!({"title": idle_expected["title"]}),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 200, "PATCH on A returned {}", resp.status);
            Ok(((), resp.body.to_string()))
        },
    )
    .await?;
    step(
        report,
        "6b-idle-propagation",
        "the edit propagates to B over the already-open idle connection",
        async {
            let seen = http::wait_for_value(
                addr_b,
                &format!("/tasks/{task1_id}"),
                &idle_expected,
                SYNC_BOUND,
            )
            .await?;
            Ok(((), seen.to_string()))
        },
    )
    .await?;

    // --- Step 7/8: graceful shutdown with an active sync connection, bounded; restart; reconnect ---
    let elapsed_a = step(
        report,
        "7a-graceful-shutdown-active-a",
        "A shuts down gracefully within a bounded time while a sync connection is active",
        async {
            let e = daemon_a.terminate_gracefully(SHUTDOWN_BOUND).await?;
            anyhow::ensure!(e <= SHUTDOWN_BOUND, "exceeded bound");
            Ok((e, format!("exited in {e:?} (bound {SHUTDOWN_BOUND:?})")))
        },
    )
    .await?;
    let elapsed_b = step(
        report,
        "7a-graceful-shutdown-active-b",
        "B shuts down gracefully within a bounded time while a sync connection is active",
        async {
            let e = daemon_b.terminate_gracefully(SHUTDOWN_BOUND).await?;
            anyhow::ensure!(e <= SHUTDOWN_BOUND, "exceeded bound");
            Ok((e, format!("exited in {e:?} (bound {SHUTDOWN_BOUND:?})")))
        },
    )
    .await?;
    step(
        report,
        "7b-relay-teardown",
        "relay observes both connections closing",
        async {
            relay3.disconnect().await?;
            Ok((
                (),
                format!("A stopped in {elapsed_a:?}, B in {elapsed_b:?}"),
            ))
        },
    )
    .await?;

    let db_path_a = daemon_a.db_path.clone();
    let db_path_b = daemon_b.db_path.clone();
    let mut daemon_a = step(
        report,
        "7c-restart-a",
        "restart A from its own durable store (final restart)",
        async {
            let d = Daemon::restart(bin, &dir_a, &db_path_a, "A")?;
            Ok((d, "restarted".to_string()))
        },
    )
    .await?;
    let mut daemon_b = step(
        report,
        "7c-restart-b",
        "restart B from its own durable store (final restart)",
        async {
            let d = Daemon::restart(bin, &dir_b, &db_path_b, "B")?;
            Ok((d, "restarted".to_string()))
        },
    )
    .await?;
    let addr_a = step(report, "7c-ready-a", "restarted A becomes ready", async {
        let a = daemon_a.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    let addr_b = step(report, "7c-ready-b", "restarted B becomes ready", async {
        let a = daemon_b.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    step(report, "7d-identity-and-state-persisted", "identity and full converged state (including the idle-propagated edit) survive the final restart", async {
        for (addr, name) in [(addr_a, "A"), (addr_b, "B")] {
            let t1 = http::get(addr, &format!("/tasks/{task1_id}"), Duration::from_secs(5)).await?;
            anyhow::ensure!(t1.body == idle_expected, "{name} lost the idle-propagated edit across restart: {}", t1.body);
        }
        Ok(((), "task1 matches the fully-converged post-conflict, post-idle-edit state on both peers".to_string()))
    }).await?;

    let relay4 = step(
        report,
        "7e-reconnect",
        "reconnect once more after restart",
        async {
            let r = Relay::connect(addr_a, addr_b).await?;
            Ok((r, "reconnected".to_string()))
        },
    )
    .await?;
    step(
        report,
        "7f-no-op-convergence",
        "reconnecting after restart is a safe no-op (already-converged state stays converged)",
        async {
            for id in [&task1_id, &task2_id, &task3_id] {
                wait_converge_equal(addr_a, addr_b, &format!("/tasks/{id}"), SYNC_BOUND).await?;
            }
            Ok((
                (),
                "all three tasks remain equal on both peers after reconnect".to_string(),
            ))
        },
    )
    .await?;
    step(
        report,
        "7g-relay-teardown",
        "disconnect before final store inspection",
        async {
            relay4.disconnect().await?;
            Ok(((), "disconnected".to_string()))
        },
    )
    .await?;
    daemon_a.terminate_gracefully(SHUTDOWN_BOUND).await.ok();
    daemon_b.terminate_gracefully(SHUTDOWN_BOUND).await.ok();
    let db_path_a = daemon_a.db_path.clone();
    let db_path_b = daemon_b.db_path.clone();
    step(
        report,
        "7h-heads-no-duplicates",
        "final heads match, and there are no missing or duplicate semantic entities",
        async {
            let summary_a = store::inspect(&db_path_a)?;
            let summary_b = store::inspect(&db_path_b)?;
            let mut expected_ids = vec![task1_id.clone(), task2_id.clone(), task3_id.clone()];
            expected_ids.sort();
            anyhow::ensure!(
                summary_a.task_uuids == expected_ids,
                "A has unexpected task set: {:?}",
                summary_a.task_uuids
            );
            anyhow::ensure!(
                summary_b.task_uuids == expected_ids,
                "B has unexpected task set: {:?}",
                summary_b.task_uuids
            );
            anyhow::ensure!(
                summary_a.root_heads == summary_b.root_heads,
                "final root heads differ"
            );
            for id in &expected_ids {
                anyhow::ensure!(
                    summary_a.task_heads[id] == summary_b.task_heads[id],
                    "final heads for {id} differ"
                );
            }
            Ok((
                (),
                format!(
                    "exactly {} tasks on both peers, no duplicates, matching heads",
                    expected_ids.len()
                ),
            ))
        },
    )
    .await?;

    Ok(())
}

/// Step 8 (first half): an HTTP-acknowledged write survives `SIGKILL` of the
/// actual daemon process, verified by restarting a fresh process against the
/// same store.
pub async fn run_crash_scenario(
    bin: &Path,
    out_dir: &Path,
    report: &mut Report,
) -> anyhow::Result<()> {
    step(
        report,
        "8-crash-workdir",
        "create isolated working directory for the crash scenario",
        async {
            std::fs::create_dir_all(out_dir).context("create dir")?;
            Ok(((), out_dir.display().to_string()))
        },
    )
    .await?;
    let dir_c = out_dir.join("peer-c");

    let mut daemon = step(report, "8a-spawn", "spawn a real daemon process", async {
        let d = Daemon::spawn(bin, &dir_c, "C")?;
        Ok((d, "spawned".to_string()))
    })
    .await?;
    let addr = step(report, "8a-ready", "peer becomes ready", async {
        let a = daemon.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    let created = step(
        report,
        "8b-http-write",
        "an HTTP write is acknowledged with 201 Created",
        async {
            let resp = http::post(
                addr,
                "/tasks",
                &new_task("Durable write before SIGKILL"),
                Duration::from_secs(5),
            )
            .await?;
            anyhow::ensure!(resp.status == 201, "expected 201, got {}", resp.status);
            Ok((resp.body.clone(), resp.body.to_string()))
        },
    )
    .await?;
    let task_id = created["uuid"]
        .as_str()
        .context("missing uuid")?
        .to_string();

    step(
        report,
        "8c-sigkill",
        "abruptly terminate the process with SIGKILL (no graceful shutdown)",
        async {
            daemon.kill_abruptly().await?;
            Ok(((), "process killed and reaped".to_string()))
        },
    )
    .await?;

    let db_path = daemon.db_path.clone();
    let mut restarted = step(
        report,
        "8d-restart",
        "start a fresh process against the same durable store",
        async {
            let d = Daemon::restart(bin, &dir_c, &db_path, "C-restarted")?;
            Ok((d, "restarted".to_string()))
        },
    )
    .await?;
    let addr2 = step(report, "8d-ready", "restarted peer becomes ready", async {
        let a = restarted.wait_ready(STARTUP_BOUND).await?;
        Ok((a, format!("bound {a}")))
    })
    .await?;
    step(
        report,
        "8e-write-survived",
        "the HTTP-acknowledged write survived the abrupt kill and is present after restart",
        async {
            let resp =
                http::get(addr2, &format!("/tasks/{task_id}"), Duration::from_secs(5)).await?;
            anyhow::ensure!(
                resp.status == 200,
                "GET after restart returned {}",
                resp.status
            );
            anyhow::ensure!(
                resp.body == created,
                "task after crash+restart ({}) does not match the acknowledgement ({})",
                resp.body,
                created
            );
            Ok(((), "task identical to the acknowledged write".to_string()))
        },
    )
    .await?;

    let _ = restarted.terminate_gracefully(SHUTDOWN_BOUND).await;
    Ok(())
}

/// Run the full acceptance scenario (both sub-scenarios) and return the
/// report regardless of outcome — callers inspect `Report::all_passed`.
pub async fn run_full(bin: &Path, out_dir: &Path) -> Report {
    let mut report = Report::new();
    let _ = run_two_process_scenario(bin, &out_dir.join("two-process"), &mut report).await;
    let _ = run_crash_scenario(bin, &out_dir.join("crash-kill"), &mut report).await;
    report
}
