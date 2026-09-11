use axum::{body::Body, http::Request};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use samgtdd::{
    service::{Envelope, Service},
    transport::{self, App},
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(app: &axum::Router, method: &str, uri: &str, body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

// A transparent network relay connects two daemon endpoints; it never reads
// document snapshots or interprets task changes.
async fn socket(app: axum::Router) -> tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream> {
    let (client, server) = tokio::io::duplex(2 * 1024 * 1024);
    tokio::spawn(async move {
        hyper::server::conn::http1::Builder::new()
            .serve_connection(
                hyper_util::rt::TokioIo::new(server),
                hyper_util::service::TowerToHyperService::new(app),
            )
            .with_upgrades()
            .await
            .unwrap();
    });
    tokio_tungstenite::client_async("ws://localhost/sync", client)
        .await
        .unwrap()
        .0
}
async fn relay(a: axum::Router, b: axum::Router, mut stop: tokio::sync::oneshot::Receiver<()>) {
    let mut a = socket(a).await;
    let mut b = socket(b).await;
    loop {
        tokio::select! {
            _ = &mut stop => break,
            msg = a.next() => {
                let msg = msg.unwrap().unwrap();
                if msg.is_text() { b.send(msg).await.unwrap(); }
            }
            msg = b.next() => {
                let msg = msg.unwrap().unwrap();
                if msg.is_text() { a.send(msg).await.unwrap(); }
            }
        }
    }
    a.close(None).await.unwrap();
    b.close(None).await.unwrap();
}
async fn wait_task(app: &axum::Router, id: &str, expected: &Value) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/tasks/{id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            if serde_json::from_slice::<Value>(&bytes).ok().as_ref() == Some(expected) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("task failed to converge");
}
#[tokio::test]
async fn http_websocket_offline_edits_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path_a = dir.path().join("a.sqlite");
    let path_b = dir.path().join("b.sqlite");
    let a = transport::router(App::new(Service::open(&path_a).unwrap()));
    let b = transport::router(App::new(Service::open(&path_b).unwrap()));
    let identity_a = request(&a, "GET", "/identity", Value::Null).await;
    let bootstrap = request(&a, "GET", "/provision", Value::Null).await;
    request(&b, "POST", "/provision", bootstrap).await;
    let identity_b = request(&b, "GET", "/identity", Value::Null).await;
    assert_ne!(identity_a["node"], identity_b["node"]);
    assert_eq!(identity_a["dataset"], identity_b["dataset"]);

    let task = request(
        &a,
        "POST",
        "/tasks",
        json!({
            "uuid": "", "title": "Buy milk", "notes": "", "status": "TODO",
            "category_id": "00000000-0000-4000-8000-000000000001",
            "project_id": null, "domain_id": null
        }),
    )
    .await;
    let id = task["uuid"].as_str().unwrap();
    let (stop, receiver) = tokio::sync::oneshot::channel();
    let connection = tokio::spawn(relay(a.clone(), b.clone(), receiver));
    wait_task(&b, id, &task).await;
    // A local edit after the connection becomes idle must notify the peer.
    let shared = request(
        &a,
        "PATCH",
        &format!("/tasks/{id}"),
        json!({"notes": "shared"}),
    )
    .await;
    wait_task(&b, id, &shared).await;
    stop.send(()).unwrap();
    connection.await.unwrap();
    request(
        &a,
        "PATCH",
        &format!("/tasks/{id}"),
        json!({"status": "DONE"}),
    )
    .await;
    request(
        &b,
        "PATCH",
        &format!("/tasks/{id}"),
        json!({"title": "Buy oat milk"}),
    )
    .await;
    let mut expected = shared;
    expected["status"] = json!("DONE");
    expected["title"] = json!("Buy oat milk");
    let (stop, receiver) = tokio::sync::oneshot::channel();
    let connection = tokio::spawn(relay(a.clone(), b.clone(), receiver));
    wait_task(&a, id, &expected).await;
    wait_task(&b, id, &expected).await;
    stop.send(()).unwrap();
    connection.await.unwrap();
    drop(a);
    drop(b);
    // Upgraded socket tasks finish after their clients close.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let a = Service::open(&path_a).unwrap();
    let b = Service::open(&path_b).unwrap();
    assert_eq!(serde_json::to_value(a.read(id).unwrap()).unwrap(), expected);
    assert_eq!(serde_json::to_value(b.read(id).unwrap()).unwrap(), expected);
    assert_eq!(serde_json::to_value(a.identity).unwrap(), identity_a);
    assert_eq!(serde_json::to_value(b.identity).unwrap(), identity_b);
}

#[test]
fn mismatched_dataset_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut service = Service::open(&dir.path().join("a.sqlite")).unwrap();
    let result = service.exchange(
        &mut Default::default(),
        Some(Envelope {
            version: 1,
            dataset: "foreign".into(),
            kind: "root_index".into(),
            id: "default".into(),
            message: vec![],
        }),
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn acknowledged_write_survives_process_kill() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    const CHILD_PATH: &str = "SAMGTD_TEST_CRASH_PATH";
    if let Ok(path) = std::env::var(CHILD_PATH) {
        let mut service = Service::open(std::path::Path::new(&path)).unwrap();
        let task = service
            .create(
                serde_json::from_value(json!({
                    "title": "Durable task", "notes": "committed", "status": "DONE",
                    "category_id": "00000000-0000-4000-8000-000000000001"
                }))
                .unwrap(),
            )
            .unwrap();
        println!(
            "ACK {}",
            json!({"identity": service.identity, "task": task})
        );
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        std::future::pending::<()>().await;
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.sqlite");
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "acknowledged_write_survives_process_kill",
            "--nocapture",
        ])
        .env(CHILD_PATH, &path)
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let acknowledged: Value = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let line = lines
                .next_line()
                .await
                .unwrap()
                .expect("child exited before acknowledgement");
            if let Some(json) = line.strip_prefix("ACK ") {
                break serde_json::from_str(json).unwrap();
            }
        }
    })
    .await
    .expect("child acknowledgement timed out");
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    let service = Service::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(&service.identity).unwrap(),
        acknowledged["identity"]
    );
    let id = acknowledged["task"]["uuid"].as_str().unwrap();
    assert_eq!(
        serde_json::to_value(service.read(id).unwrap()).unwrap(),
        acknowledged["task"]
    );
}

#[tokio::test]
async fn shutdown_ends_an_active_sync_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = App::new(Service::open(&dir.path().join("shutdown.sqlite")).unwrap());
    let mut client = socket(transport::router(state.clone())).await;
    assert!(client.next().await.unwrap().unwrap().is_text());
    state.shutdown();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while let Some(Ok(message)) = client.next().await {
            if message.is_close() {
                break;
            }
        }
    })
    .await
    .expect("sync session did not end on shutdown");
}
