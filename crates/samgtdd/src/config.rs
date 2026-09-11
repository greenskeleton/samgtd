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
}

impl Config {
    pub fn from_env() -> Self {
        let ip = std::env::var("SAMGTD_BIND_IP")
            .ok()
            .and_then(|v| v.parse::<IpAddr>().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

        let port = std::env::var("SAMGTD_BIND_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let db_path = std::env::var("SAMGTD_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DB_PATH));

        Self {
            bind_addr: SocketAddr::new(ip, port),
            db_path,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT),
            db_path: PathBuf::from(DEFAULT_DB_PATH),
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
