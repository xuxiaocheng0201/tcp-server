use std::sync::RwLock;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ServerConfiguration {
    pub addr: String,
    pub connect_sec: u64,
    pub idle_sec: u64,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        Self {
            addr: "localhost:0".to_string(),
            connect_sec: 30,
            idle_sec: 60,
        }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<ServerConfiguration> = RwLock::new(ServerConfiguration::default());
}

#[inline]
pub fn set_server_config(config: ServerConfiguration) {
    let mut c = CONFIG.write().unwrap();
    *c = config;
}

#[inline]
pub fn get_server_config() -> ServerConfiguration {
    let c = CONFIG.read().unwrap();
    (*c).clone()
}

#[inline]
pub fn get_server_addr() -> String {
    let c = CONFIG.read().unwrap();
    (*c).addr.clone()
}

#[inline]
pub fn get_server_connect_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).connect_sec
}

#[inline]
pub fn get_server_idle_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).idle_sec
}
