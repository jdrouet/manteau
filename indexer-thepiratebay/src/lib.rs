use manteau_indexer_prelude::{Category, Indexer, IndexerBuilder, IndexerResult};

mod common;
mod feed;
mod search;

const MUSIC_CATEGORIES: [u16; 2] = [101, 104];
const MOVIE_CATEGORIES: [u16; 3] = [201, 202, 207];
const TVSHOW_CATEGORIES: [u16; 2] = [205, 208];
const BOOK_CATEGORIES: [u16; 1] = [601];

const BASE_URL: &str = "https://apibay.org";
pub const NAME: &str = "thepiratebay";

#[derive(Debug, serde::Deserialize)]
pub struct IndexerBitsearchConfig {
    #[serde(default = "IndexerBitsearchConfig::default_base_url")]
    pub base_url: String,
}

impl IndexerBitsearchConfig {
    fn default_base_url() -> String {
        BASE_URL.into()
    }
}

impl IndexerBuilder for IndexerBitsearchConfig {
    fn build(self, name: String) -> Box<dyn Indexer + Send + Sync + 'static> {
        tracing::info!("building {NAME} indexer named {name:?}");
        Box::new(IndexerBitsearch {
            name,
            base_url: self.base_url,
        })
    }
}

#[derive(Debug)]
pub struct IndexerBitsearch {
    name: String,
    base_url: String,
}

impl Default for IndexerBitsearch {
    fn default() -> Self {
        Self::new(BASE_URL)
    }
}

impl IndexerBitsearch {
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        Self {
            name: "bitsearch".into(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait::async_trait]
impl Indexer for IndexerBitsearch {
    async fn search(&self, query: &str) -> IndexerResult {
        tracing::debug!("{} searching {query:?}", self.name);
        search::execute(&self.base_url, query, 0).await
    }

    async fn feed(&self, category: Category) -> IndexerResult {
        tracing::debug!("{} fetching feed for {category:?}", self.name);
        let cats: &[u16] = match category {
            Category::Audio | Category::Music => &MUSIC_CATEGORIES,
            Category::Movie => &MOVIE_CATEGORIES,
            Category::Tv => &TVSHOW_CATEGORIES,
            Category::Book => &BOOK_CATEGORIES,
        };
        feed::execute(&self.base_url, cats).await
    }
}
