use std::env;
use std::net::IpAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionMode {
    UnixSocket,
    Tcp,
}

impl FromStr for ConnectionMode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNIX_SOCK" => Ok(ConnectionMode::UnixSocket),
            "TCP" => Ok(ConnectionMode::Tcp),
            _ => Err(anyhow::anyhow!("Invalid connection mode: {}", s)),
        }
    }
}

pub struct Settings {
    pub key_expiry: Duration,
    pub connection_mode: ConnectionMode,
    pub unix_sock_path: String,
    pub tcp_port: u16,
    pub tcp_host: IpAddr,
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        let key_expiry = env::var("FSDB_KEY_EXPIRY")
            .map(|v| v.parse::<u64>().map(Duration::from_secs))
            .unwrap_or(Ok(Duration::from_secs(150)))?;

        let connection_mode = env::var("FSDB_CONNECTION_MODE")
            .map(|v| ConnectionMode::from_str(&v))
            .unwrap_or(Ok(ConnectionMode::UnixSocket))?;

        let unix_sock_path =
            env::var("FSDB_UNIX_SOCK_PATH").unwrap_or_else(|_| "/tmp/fsdb.sock".to_string());

        let tcp_port = env::var("FSDB_TCP_PORT")
            .map(|v| v.parse::<u16>())
            .unwrap_or(Ok(1273))?;

        let tcp_host = env::var("FSDB_TCP_HOST")
            .map(|v| IpAddr::from_str(&v))
            .unwrap_or(Ok(IpAddr::from_str("127.0.0.1").unwrap()))?;

        Ok(Self {
            key_expiry,
            connection_mode,
            unix_sock_path,
            tcp_port,
            tcp_host,
        })
    }

    pub fn get() -> &'static Self {
        static SETTINGS: LazyLock<Settings> =
            LazyLock::new(|| Settings::from_env().expect("Failed to load settings"));

        SETTINGS.deref()
    }
}
