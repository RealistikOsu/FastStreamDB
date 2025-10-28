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
        let key_expiry = Duration::from_secs(env::var("FSDB_KEY_EXPIRY")?.parse::<u64>()?);
        let connection_mode = ConnectionMode::from_str(&env::var("FSDB_CONNECTION_MODE")?)?;
        let unix_sock_path = env::var("FSDB_UNIX_SOCK_PATH")?;
        let tcp_port = env::var("FSDB_TCP_PORT")?.parse::<u16>()?;
        let tcp_host = IpAddr::from_str(&env::var("FSDB_TCP_HOST")?)?;

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
