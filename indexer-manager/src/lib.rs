pub use bytesize;

use manteau_indexer_prelude::{Category, Indexer, IndexerBuilder, IndexerResult};
use std::collections::HashMap;

mod bitsearch;
mod i1337x;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum IndexerConfig {
    #[serde(rename = "1337x")]
    I1337x(i1337x::Indexer1337xConfig),
    #[serde(rename = "bitsearch")]
    Bitsearch(bitsearch::IndexerBitsearchConfig),
}

impl IndexerBuilder for IndexerConfig {
    fn build(self, name: String) -> Box<dyn Indexer + Send + Sync + 'static> {
        match self {
            Self::Bitsearch(inner) => inner.build(name),
            Self::I1337x(inner) => inner.build(name),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct IndexerManagerConfig(HashMap<String, IndexerConfig>);

impl IndexerManagerConfig {
    pub fn build(self) -> IndexerManager {
        IndexerManager {
            indexers: self
                .0
                .into_iter()
                .map(|(name, config)| config.build(name))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct IndexerManager {
    indexers: Vec<Box<dyn Indexer + Send + Sync + 'static>>,
}

impl Default for IndexerManager {
    fn default() -> Self {
        Self {
            indexers: vec![
                Box::<i1337x::Indexer1337x>::default(),
                Box::<bitsearch::IndexerBitsearch>::default(),
            ],
        }
    }
}

impl IndexerManager {
    pub fn with_indexer<I: Indexer + Send + Sync + 'static>(indexer: I) -> Self {
        Self {
            indexers: vec![Box::new(indexer)],
        }
    }

    pub async fn search(&self, query: &str) -> IndexerResult {
        let items =
            futures::future::join_all(self.indexers.iter().map(|idx| idx.search(query))).await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }

    pub async fn feed(&self, category: Category) -> IndexerResult {
        let items =
            futures::future::join_all(self.indexers.iter().map(|idx| idx.feed(category))).await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }
}
