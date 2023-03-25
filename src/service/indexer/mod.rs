use std::sync::Arc;

pub mod bitsearch;
pub mod i1337x;
pub mod prelude;

#[derive(Clone, Debug, Default)]
pub struct IndexerManager(Arc<IndexerManagerInner>);

#[derive(Debug, Default)]
pub struct IndexerManagerInner {
    indexer_1337x: i1337x::Indexer1337x,
    indexer_bitsearch: bitsearch::IndexerBitsearch,
}

impl IndexerManager {
    pub async fn search(&self, query: &str) -> prelude::IndexerResult {
        let (i1337x, bitsearch) = tokio::join!(
            self.0.indexer_1337x.search(query),
            self.0.indexer_bitsearch.search(query),
        );
        i1337x.merge(bitsearch)
    }

    pub async fn feed(&self, category: prelude::Category) -> prelude::IndexerResult {
        let (i1337x, bitsearch) = tokio::join!(
            self.0.indexer_1337x.feed(category),
            self.0.indexer_bitsearch.feed(category),
        );
        i1337x.merge(bitsearch)
    }
}
