pub use moka::future::Cache;
use std::time::Duration;

#[derive(Debug, serde::Deserialize)]
pub struct CacheConfig {
    #[serde(default = "CacheConfig::default_capacity")]
    pub capacity: u64,
    #[serde(default = "CacheConfig::default_ttl")]
    pub ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: Self::default_capacity(),
            ttl: Self::default_ttl(),
        }
    }
}

impl CacheConfig {
    fn default_capacity() -> u64 {
        100
    }

    fn default_ttl() -> u64 {
        60
    }

    pub fn build(self) -> Cache<String, String> {
        Cache::builder()
            .max_capacity(self.capacity)
            .time_to_live(Duration::from_secs(self.ttl))
            .build()
    }
}

#[cfg(test)]
pub fn build() -> std::sync::Arc<Cache<String, String>> {
    std::sync::Arc::new(CacheConfig::default().build())
}
