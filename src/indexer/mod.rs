pub mod i1337x;
pub mod prelude;

#[derive(Debug)]
pub struct IndexerManager {
    indexers: Vec<Box<dyn prelude::Indexer>>,
}

impl Default for IndexerManager {
    fn default() -> Self {
        Self {
            indexers: vec![Box::<i1337x::Indexer1337x>::default()],
        }
    }
}

impl IndexerManager {
    pub async fn search(&self, query: &str) -> prelude::SearchResult {
        let mut results = prelude::SearchResult::default();
        for index in self.indexers.iter() {
            let result = index.search(query).await;
            results.merge(result);
        }
        results
    }
}
