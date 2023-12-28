use std::sync::RwLock;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Configuration {
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser_addr", deserialize_with = "de_addr"))]
    pub addr: Result<String, &'static str>,
    pub connect_sec: u64,
    pub idle_sec: u64,
}

#[cfg(feature = "serde")]
fn ser_addr<S>(addr: &Result<String, &'static str>, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    match addr {
        Ok(a) => serializer.serialize_str(a),
        Err(a) => serializer.serialize_str(a),
    }
}

#[cfg(feature = "serde")]
fn de_addr<'de, D>(deserializer: D) -> Result<Result<String, &'static str>, D::Error> where D: serde::Deserializer<'de> {
    use serde::Deserialize;
    Ok(Ok(String::deserialize(deserializer)?))
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            addr: Err("localhost:0"),
            connect_sec: 30,
            idle_sec: 60,
        }
    }
}

static CONFIG: RwLock<Configuration> = RwLock::new(Configuration {
    addr: Err("localhost:0"),
    connect_sec: 30,
    idle_sec: 60,
});

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
    match &(*c).addr {
        Ok(a) => a.clone(),
        Err(a) => a.to_string(),
    }
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
