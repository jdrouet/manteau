pub mod bitsearch;
pub mod i1337x;
pub mod prelude;

#[derive(Debug)]
pub struct IndexerManager {
    indexers: Vec<Box<dyn prelude::Indexer>>,
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
    pub async fn search(&self, query: &str) -> prelude::SearchResult {
        let calls = self.indexers.iter().map(|indexer| indexer.search(query));
        let items = futures::future::join_all(calls).await;
        items
            .into_iter()
            .fold(prelude::SearchResult::default(), |res, item| {
                res.merge(item)
            })
    }
}
