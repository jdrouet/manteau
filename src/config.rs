use manteau_indexer_manager::IndexerManagerConfig;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub indexers: IndexerManagerConfig,
    #[serde(default)]
    pub torznab: crate::service::torznab::TorznabConfig,
    #[serde(default)]
    pub cache: crate::service::cache::CacheConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let path = std::env::var("CONFIG_FILE")
            .ok()
            .unwrap_or_else(|| String::from("./config.toml"));
        Self::from_path(PathBuf::from(path))
    }

    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        std::fs::read_to_string(path)
            .map_err(|err| err.to_string())
            .and_then(|inner| Self::from_str(inner.as_str()))
    }

    pub fn from_str(inner: &str) -> Result<Self, String> {
        toml::from_str(inner).map_err(|err| err.to_string())
    }
}
