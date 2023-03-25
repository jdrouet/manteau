use prelude::IndexerResult;
use std::sync::Arc;

mod bitsearch;
mod i1337x;
pub mod prelude;

use prelude::Indexer;

#[derive(Clone, Debug, Default)]
pub struct IndexerManager(Arc<IndexerManagerInner>);

#[derive(Debug, Default)]
struct IndexerManagerInner {
    indexer_1337x: i1337x::Indexer1337x,
    indexer_bitsearch: bitsearch::IndexerBitsearch,
}

impl IndexerManager {
    pub async fn search(&self, query: &str) -> prelude::IndexerResult {
        let items = futures::future::join_all([
            self.0.indexer_1337x.search(query),
            self.0.indexer_bitsearch.search(query),
        ])
        .await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }

    pub async fn feed(&self, category: prelude::Category) -> prelude::IndexerResult {
        let items = futures::future::join_all([
            self.0.indexer_1337x.feed(category),
            self.0.indexer_bitsearch.feed(category),
        ])
        .await;
        items
            .into_iter()
            .fold(IndexerResult::default(), |res, item| res.merge(item))
    }
}
