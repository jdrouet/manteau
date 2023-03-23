pub use bytesize;

use prelude::{IndexerBuilder, IndexerResult};
use std::{collections::HashMap, sync::Arc};

mod bitsearch;
mod i1337x;
pub mod prelude;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum IndexerConfig {
    #[serde(rename = "1337x")]
    I1337x(i1337x::Indexer1337xConfig),
    #[serde(rename = "bitsearch")]
    Bitsearch(bitsearch::IndexerBitsearchConfig),
}

impl IndexerBuilder for IndexerConfig {
    fn build(self, name: String) -> Box<dyn prelude::Indexer + Send + Sync + 'static> {
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
        IndexerManager(Arc::new(IndexerManagerInner {
            indexers: self
                .0
                .into_iter()
                .map(|(name, config)| config.build(name))
                .collect(),
        }))
    }
}

#[derive(Clone, Debug, Default)]
pub struct IndexerManager(Arc<IndexerManagerInner>);

impl IndexerManager {
    pub fn with_indexer<I: prelude::Indexer + Send + Sync + 'static>(indexer: I) -> Self {
        Self(Arc::new(IndexerManagerInner {
            indexers: vec![Box::new(indexer)],
        }))
    }
}

#[derive(Debug)]
struct IndexerManagerInner {
    indexers: Vec<Box<dyn prelude::Indexer + Send + Sync + 'static>>,
}

impl Default for IndexerManagerInner {
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
    pub async fn search(&self, query: &str) -> prelude::IndexerResult {
        let items =
            futures::future::join_all(self.0.indexers.iter().map(|idx| idx.search(query))).await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }

    pub async fn feed(&self, category: prelude::Category) -> prelude::IndexerResult {
        let items =
            futures::future::join_all(self.0.indexers.iter().map(|idx| idx.feed(category))).await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }
}
