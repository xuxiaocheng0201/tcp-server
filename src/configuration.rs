use std::sync::RwLock;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Configuration {
    pub addr: String,
    pub connect_sec: u64,
    pub idle_sec: u64,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            addr: "localhost:0".to_string(),
            connect_sec: 30,
            idle_sec: 60,
        }
    }
}

lazy_static! {
    static ref CONFIG: RwLock<Configuration> = RwLock::new(Configuration::default());
}

#[inline]
pub fn set_config(config: Configuration) {
    let mut c = CONFIG.write().unwrap();
    *c = config;
}

#[inline]
pub fn get_config() -> Configuration {
    let c = CONFIG.read().unwrap();
    (*c).clone()
}

#[inline]
pub fn get_addr() -> String {
    let c = CONFIG.read().unwrap();
    (*c).addr.clone()
}

#[inline]
pub fn get_connect_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).connect_sec
}

#[inline]
pub fn get_idle_sec() -> u64 {
    let c = CONFIG.read().unwrap();
    (*c).idle_sec
}
