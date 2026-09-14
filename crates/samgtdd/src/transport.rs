use crate::service::{Envelope, Provision, Service, Session, TaskPatch};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use samgtd_crdt::TaskFields;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

#[derive(Clone)]
pub struct App {
    service: Arc<Mutex<Service>>,
    changed: watch::Sender<u64>,
    shutdown: watch::Sender<bool>,
}
impl App {
    pub fn new(service: Service) -> Self {
        Self {
            service: Arc::new(Mutex::new(service)),
            changed: watch::channel(0).0,
            shutdown: watch::channel(false).0,
        }
    }
    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
    }
    async fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Service) -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<T> {
        let service = self.service.clone();
        tokio::task::spawn_blocking(move || {
            let mut service = service
                .lock()
                .map_err(|_| anyhow::anyhow!("application lock poisoned"))?;
            f(&mut service)
        })
        .await?
    }
    fn notify(&self) {
        self.changed.send_modify(|v| *v = v.wrapping_add(1));
    }
}
type Error = (StatusCode, String);
fn error(err: anyhow::Error) -> Error {
    tracing::warn!(error = %err, "operation failed");
    (StatusCode::BAD_REQUEST, err.to_string())
}
pub fn router(app: App) -> Router {
    crate::build_router()
        .route("/identity", get(identity))
        .route("/provision", get(provision).post(join))
        .route("/tasks", post(create))
        .route("/tasks/{id}", get(read).patch(update))
        .route("/sync", get(upgrade))
        .with_state(app)
}
async fn identity(State(app): State<App>) -> Result<impl IntoResponse, Error> {
    app.call(|s| Ok(Json(s.identity.clone())))
        .await
        .map_err(error)
}
async fn provision(State(app): State<App>) -> Result<impl IntoResponse, Error> {
    app.call(|s| Ok(Json(s.provision()?))).await.map_err(error)
}
async fn join(
    State(app): State<App>,
    Json(provision): Json<Provision>,
) -> Result<StatusCode, Error> {
    app.call(move |s| s.join(provision)).await.map_err(error)?;
    app.notify();
    Ok(StatusCode::NO_CONTENT)
}
async fn create(
    State(app): State<App>,
    Json(fields): Json<TaskFields>,
) -> Result<impl IntoResponse, Error> {
    let fields = app.call(move |s| s.create(fields)).await.map_err(error)?;
    app.notify();
    Ok((StatusCode::CREATED, Json(fields)))
}
async fn read(State(app): State<App>, Path(id): Path<String>) -> Result<impl IntoResponse, Error> {
    app.call(move |s| Ok(Json(s.read(&id)?)))
        .await
        .map_err(error)
}
async fn update(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(patch): Json<TaskPatch>,
) -> Result<impl IntoResponse, Error> {
    let fields = app
        .call(move |s| s.update(&id, patch))
        .await
        .map_err(error)?;
    app.notify();
    Ok(Json(fields))
}
async fn upgrade(State(app): State<App>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.max_message_size(1024 * 1024)
        .max_frame_size(1024 * 1024)
        .on_upgrade(move |socket| async move {
            if let Err(err) = sync_socket(app, socket).await {
                tracing::debug!(error = %err, "sync connection ended");
            }
        })
}
async fn sync_socket(app: App, mut socket: WebSocket) -> anyhow::Result<()> {
    let mut changed = app.changed.subscribe();
    let mut shutdown = app.shutdown.subscribe();
    let mut states = Session::new();
    let dataset = app.call(|s| Ok(s.identity.dataset.clone())).await?;
    let mut incoming = None;
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }
        changed.borrow_and_update();
        let received = incoming.is_some();
        let expected = dataset.clone();
        let (next_states, messages) = app
            .call(move |s| {
                anyhow::ensure!(s.identity.dataset == expected, "dataset changed");
                let messages = s.exchange(&mut states, incoming)?;
                Ok((states, messages))
            })
            .await?;
        states = next_states;
        if received {
            app.notify();
        }
        for message in messages {
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                socket.send(Message::Text(serde_json::to_string(&message)?.into())),
            )
            .await??;
        }
        incoming = tokio::select! {
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => Some(serde_json::from_str::<Envelope>(&text)?),
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Ok(Message::Ping(bytes))) => { socket.send(Message::Pong(bytes)).await?; None },
                Some(Ok(Message::Pong(_))) => None,
                _ => anyhow::bail!("expected JSON text frame"),
            },
            _ = changed.changed() => None,
            _ = shutdown.changed() => return Ok(()),
        };
    }
}
