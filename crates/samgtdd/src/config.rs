use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

const DEFAULT_PORT: u16 = 4173;
const DEFAULT_DB_PATH: &str = "samgtd-data.sqlite";

/// Daemon configuration.
///
/// Network defaults to loopback-only per
/// `docs/adr/0002-private-network.md`. LAN binding must be explicit
/// (`SAMGTD_BIND_IP`) — never inferred.
///
/// `db_path` is a samgtd-owned database, separate from
/// `.local/current-gtd.sqlite` (see `docs/adr/0003-existing-database-coexistence.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub db_path: PathBuf,
    /// Optional path to announce the actually-bound listener address on
    /// (written atomically after a successful `TcpListener::bind`). Mainly
    /// useful together with `SAMGTD_BIND_PORT=0`: callers that need an
    /// OS-assigned ephemeral port (tests, running multiple instances) can
    /// read the real address back here instead of guessing/reserving one.
    pub ready_path: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let ip = std::env::var("SAMGTD_BIND_IP")
            .map(|v| v.parse::<IpAddr>())
            .unwrap_or(Ok(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let port = std::env::var("SAMGTD_BIND_PORT")
            .map(|v| v.parse::<u16>())
            .unwrap_or(Ok(DEFAULT_PORT))?;

        let db_path = std::env::var("SAMGTD_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DB_PATH));
        let ready_path = std::env::var("SAMGTD_READY_FILE").ok().map(PathBuf::from);

        Ok(Self {
            bind_addr: SocketAddr::new(ip, port),
            db_path,
            ready_path,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT),
            db_path: PathBuf::from(DEFAULT_DB_PATH),
            ready_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_binds_to_loopback() {
        assert!(Config::default().bind_addr.ip().is_loopback());
    }
}
