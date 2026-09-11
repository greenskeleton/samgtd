//! Spawns and controls real `samgtdd` binaries as child processes, each
//! bound to its own loopback TCP port and its own temporary SQLite store.
//!
//! Port allocation is race-free: daemons are always started with
//! `SAMGTD_BIND_PORT=0` (OS-assigned ephemeral port) and report the address
//! they actually bound via a `SAMGTD_READY_FILE` the daemon writes after a
//! successful `TcpListener::bind`. There is no separate "reserve a port,
//! hope it's still free" step and therefore nothing to retry on bind
//! collision.

use anyhow::{ensure, Context};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};

const CAPTURED_LINES: usize = 500;

#[derive(Clone, Default)]
struct LineBuf(Arc<Mutex<Vec<String>>>);

impl LineBuf {
    fn push(&self, line: String) {
        let mut buf = self.0.lock().expect("line buffer poisoned");
        buf.push(line);
        let len = buf.len();
        if len > CAPTURED_LINES {
            buf.drain(0..len - CAPTURED_LINES);
        }
    }
    fn snapshot(&self) -> Vec<String> {
        self.0.lock().expect("line buffer poisoned").clone()
    }
}

fn spawn_capture<R>(pipe: R, buf: LineBuf)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            buf.push(line);
        }
    });
}

/// A `samgtdd` child process, plus everything needed to talk to it and, on
/// failure, explain why.
pub struct Daemon {
    pub name: String,
    pub dir: PathBuf,
    pub db_path: PathBuf,
    ready_path: PathBuf,
    bin: PathBuf,
    child: Option<Child>,
    stdout: LineBuf,
    stderr: LineBuf,
}

pub struct CapturedOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl CapturedOutput {
    pub fn render(&self) -> String {
        format!(
            "--- stdout (last {} lines) ---\n{}\n--- stderr (last {} lines) ---\n{}",
            self.stdout.len(),
            self.stdout.join("\n"),
            self.stderr.len(),
            self.stderr.join("\n"),
        )
    }
}

impl Daemon {
    /// Spawn a new `samgtdd` process under `dir` (created if needed), using
    /// `dir/store.sqlite` as its database and loopback port 0.
    pub fn spawn(bin: &Path, dir: &Path, name: &str) -> anyhow::Result<Self> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("create working directory {}", dir.display()))?;
        let db_path = dir.join("store.sqlite");
        let ready_path = dir.join("ready");
        let _ = std::fs::remove_file(&ready_path);
        Self::spawn_with_store(bin, dir, &db_path, &ready_path, name)
    }

    /// Spawn a `samgtdd` process against an already-existing store path
    /// (used to restart a daemon on the same durable state).
    pub fn restart(bin: &Path, dir: &Path, db_path: &Path, name: &str) -> anyhow::Result<Self> {
        let ready_path = dir.join("ready");
        let _ = std::fs::remove_file(&ready_path);
        Self::spawn_with_store(bin, dir, db_path, &ready_path, name)
    }

    fn spawn_with_store(
        bin: &Path,
        dir: &Path,
        db_path: &Path,
        ready_path: &Path,
        name: &str,
    ) -> anyhow::Result<Self> {
        let mut command = Command::new(bin);
        command
            .env("SAMGTD_BIND_IP", "127.0.0.1")
            .env("SAMGTD_BIND_PORT", "0")
            .env("SAMGTD_DB_PATH", db_path)
            .env("SAMGTD_READY_FILE", ready_path)
            .env("RUST_LOG", "info")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .with_context(|| format!("spawn {name} ({})", bin.display()))?;
        let stdout = LineBuf::default();
        let stderr = LineBuf::default();
        spawn_capture(child.stdout.take().expect("piped stdout"), stdout.clone());
        spawn_capture(child.stderr.take().expect("piped stderr"), stderr.clone());
        Ok(Self {
            name: name.to_string(),
            dir: dir.to_path_buf(),
            db_path: db_path.to_path_buf(),
            ready_path: ready_path.to_path_buf(),
            bin: bin.to_path_buf(),
            child: Some(child),
            stdout,
            stderr,
        })
    }

    pub fn captured_output(&self) -> CapturedOutput {
        CapturedOutput {
            stdout: self.stdout.snapshot(),
            stderr: self.stderr.snapshot(),
        }
    }

    /// Poll for the ready file (bounded, no fixed sleep) and return the real
    /// loopback address the daemon bound. Fails fast with captured
    /// diagnostics if the process exits first.
    pub async fn wait_ready(&mut self, bound: Duration) -> anyhow::Result<SocketAddr> {
        let start = Instant::now();
        loop {
            if let Ok(contents) = tokio::fs::read_to_string(&self.ready_path).await {
                let trimmed = contents.trim();
                if let Ok(addr) = trimmed.parse::<SocketAddr>() {
                    return Ok(addr);
                }
            }
            if let Some(child) = self.child.as_mut() {
                if let Some(status) = child.try_wait().context("poll child status")? {
                    anyhow::bail!(
                        "{} exited before becoming ready ({status})\n{}",
                        self.name,
                        self.captured_output().render()
                    );
                }
            }
            if start.elapsed() > bound {
                anyhow::bail!(
                    "{} did not become ready within {bound:?}\n{}",
                    self.name,
                    self.captured_output().render()
                );
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Send SIGTERM and wait (bounded) for a clean, successful exit — the
    /// daemon's own graceful-shutdown path (axum's `with_graceful_shutdown`
    /// plus a 10s drain), not `SIGKILL`.
    #[cfg(unix)]
    pub async fn terminate_gracefully(&mut self, bound: Duration) -> anyhow::Result<Duration> {
        let child = self.child.as_mut().context("already terminated")?;
        let pid = child.id().context("process has no pid (already exited)")?;
        let start = Instant::now();
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .await
            .context("invoke kill -TERM")?;
        ensure!(status.success(), "kill -TERM {pid} failed: {status}");
        let exit = tokio::time::timeout(bound, child.wait())
            .await
            .with_context(|| {
                format!(
                    "{} did not exit within {bound:?} of SIGTERM\n{}",
                    self.name,
                    self.captured_output().render()
                )
            })?
            .context("wait for graceful exit")?;
        let elapsed = start.elapsed();
        ensure!(
            exit.success(),
            "{} exited with {exit} after SIGTERM (not a clean shutdown)\n{}",
            self.name,
            self.captured_output().render()
        );
        Ok(elapsed)
    }

    /// Forcefully kill the process (`SIGKILL`) without giving it a chance to
    /// flush or shut down cleanly, to test durability of already-acknowledged
    /// writes.
    pub async fn kill_abruptly(&mut self) -> anyhow::Result<()> {
        let child = self.child.as_mut().context("already terminated")?;
        child.kill().await.context("SIGKILL")?;
        let _ = child.wait().await;
        Ok(())
    }

    pub fn bin(&self) -> &Path {
        &self.bin
    }
}
