use std::net::SocketAddr;

/// Studio runtime configuration, read from the environment.
pub struct StudioConfig {
    /// Where the studio binds — loopback by default (never public).
    pub addr: SocketAddr,
    /// The configured owner secret; `None` (unset/blank) means nobody can log in.
    pub owner_secret: Option<String>,
}

impl StudioConfig {
    /// Read configuration from the environment with safe defaults.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            addr: studio_addr(),
            owner_secret: oathstar_auth::owner_secret_from_env(),
        }
    }
}

/// The studio bind address: `OATHSTAR_STUDIO_ADDR` or the loopback default.
fn studio_addr() -> SocketAddr {
    std::env::var("OATHSTAR_STUDIO_ADDR")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_else(default_studio_addr)
}

/// The default loopback bind — `127.0.0.1:7879`, never public.
fn default_studio_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 7879))
}

#[cfg(test)]
mod tests {
    use super::{default_studio_addr, StudioConfig};

    #[test]
    fn default_bind_is_loopback() {
        // T13/REQ-005: the studio never defaults to a public interface.
        let addr = default_studio_addr();
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.port(), 7879);
        assert_eq!(addr, "127.0.0.1:7879".parse().expect("a valid socket addr"));
    }

    #[test]
    fn from_env_reads_addr_and_secret() {
        // Sole test touching OATHSTAR_STUDIO_ADDR + OATHSTAR_OWNER_PASSWORD in
        // this binary → race-free. Covers the default and the override arms.
        std::env::remove_var("OATHSTAR_STUDIO_ADDR");
        std::env::remove_var("OATHSTAR_OWNER_PASSWORD");
        let bare = StudioConfig::from_env();
        assert_eq!(bare.addr, default_studio_addr());
        assert_eq!(bare.owner_secret, None);

        std::env::set_var("OATHSTAR_STUDIO_ADDR", "127.0.0.1:9191");
        std::env::set_var("OATHSTAR_OWNER_PASSWORD", "pw");
        let set = StudioConfig::from_env();
        std::env::remove_var("OATHSTAR_STUDIO_ADDR");
        std::env::remove_var("OATHSTAR_OWNER_PASSWORD");
        assert_eq!(set.addr.port(), 9191);
        assert_eq!(set.owner_secret.as_deref(), Some("pw"));
    }
}
