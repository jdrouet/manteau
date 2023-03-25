pub use bytesize;

use prelude::IndexerResult;
use std::sync::Arc;

mod bitsearch;
mod i1337x;
pub mod prelude;

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
                Box::new(i1337x::Indexer1337x::default()),
                Box::new(bitsearch::IndexerBitsearch::default()),
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
