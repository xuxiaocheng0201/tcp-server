//! Global configuration for this crate.
//!
//! You may change the configuration by calling `set_config` function.
//!
//! # Example
//! ```rust
//! use tcp_server::config::{ServerConfig, set_config};
//!
//! # fn main() {
//! set_config(ServerConfig::default());
//! # }
//! ```

use std::sync::RwLock;
use lazy_static::lazy_static;

/// Global configuration.
///
/// # Example
/// ```rust
/// use tcp_server::config::ServerConfig;
///
/// # fn main() {
/// let config = ServerConfig::default();
/// # let _ = config;
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ServerConfig {
    /// `addr` is the address the server bind.
    /// Default value is `localhost:0`.
    ///
    /// # Example
    /// ```rust
    /// use tcp_server::config::ServerConfig;
    ///
    /// # fn main() {
    /// let config = ServerConfig {
    ///     addr: "localhost:0".to_string(),
    ///     ..ServerConfig::default()
    /// };
    /// # let _ = config;
    /// # }
    /// ```
    pub addr: String,

    /// `connect_sec` is the timeout of connecting from the client.
    /// Default value is `30`.
    ///
    /// # Example
    /// ```rust
    /// use tcp_server::config::ServerConfig;
    ///
    /// # fn main() {
    /// let config = ServerConfig {
    ///     connect_sec: 30,
    ///     ..ServerConfig::default()
    /// };
    /// # let _ = config;
    /// # }
    /// ```
    pub connect_sec: u64,

    /// `idle_sec` is the timeout of sending/receiving message.
    /// Default value is `30`.
    ///
    /// # Example
    /// ```rust
    /// use tcp_server::config::ServerConfig;
    ///
    /// # fn main() {
    /// let config = ServerConfig {
    ///     idle_sec: 30,
    ///     ..ServerConfig::default()
    /// };
    /// # let _ = config;
    /// # }
    /// ```
    pub idle_sec: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "localhost:0".to_string(),
            connect_sec: 30,
            idle_sec: 30,
        }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ServerConfig> = RwLock::new(ServerConfig::default());
}

/// Sets the global configuration.
///
/// This function is recommended to only be called once during initialization.
///
/// # Example
/// ```rust
/// use tcp_server::config::{ServerConfig, set_config};
///
/// # fn main() {
/// set_config(ServerConfig::default());
/// # }
/// ```
#[inline]
pub fn set_config(config: ServerConfig) {
    let mut c = CONFIG.write().unwrap();
    *c = config;
}

/// Gets the global configuration.
///
/// # Example
/// ```rust
/// use tcp_server::config::get_config;
///
/// # fn main() {
/// let config = get_config();
/// # let _ = config;
/// # }
/// ```
#[inline]
pub fn get_config() -> ServerConfig {
    let c = CONFIG.read().unwrap();
    (*c).clone()
}

/// A cheaper shortcut of
/// ```rust,ignore
/// get_config().addr
/// ```
#[inline]
pub fn get_addr() -> String {
    let c = CONFIG.read().unwrap();
    (*c).addr.clone()
}

/// A cheaper shortcut of
/// ```rust,ignore
/// get_config().connect_sec
/// ```
#[inline]
pub fn get_connect_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).connect_sec
}

/// A cheaper shortcut of
/// ```rust,ignore
/// get_config().idle_sec
/// ```
#[inline]
pub fn get_idle_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).idle_sec
}
