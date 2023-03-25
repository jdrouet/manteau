use manteau_indexer_prelude::{Category, Indexer, IndexerBuilder, IndexerResult};

mod common;
mod feed;
mod search;

const MUSIC_CATEGORIES: [u16; 2] = [101, 104];
const MOVIE_CATEGORIES: [u16; 3] = [201, 202, 207];
const TVSHOW_CATEGORIES: [u16; 2] = [205, 208];
const BOOK_CATEGORIES: [u16; 1] = [601];

const API_URL: &str = "https://apibay.org";
const BASE_URL: &str = "https://thepiratebay.org";
pub const NAME: &str = "thepiratebay";

#[derive(Debug, serde::Deserialize)]
pub struct IndexerThePirateBayConfig {
    #[serde(default = "IndexerThePirateBayConfig::default_api_url")]
    pub api_url: String,
    #[serde(default = "IndexerThePirateBayConfig::default_base_url")]
    pub base_url: String,
}

impl IndexerThePirateBayConfig {
    fn default_api_url() -> String {
        API_URL.into()
    }
    fn default_base_url() -> String {
        BASE_URL.into()
    }
}

impl IndexerBuilder for IndexerThePirateBayConfig {
    fn build(self, name: String) -> Box<dyn Indexer + Send + Sync + 'static> {
        tracing::info!("building {NAME} indexer named {name:?}");
        Box::new(IndexerThePirateBay {
            name,
            api_url: self.api_url,
            base_url: self.base_url,
        })
    }
}

#[derive(Debug)]
pub struct IndexerThePirateBay {
    name: String,
    api_url: String,
    base_url: String,
}

impl Default for IndexerThePirateBay {
    fn default() -> Self {
        Self::new(API_URL, BASE_URL)
    }
}

impl IndexerThePirateBay {
    pub fn new<A: Into<String>, B: Into<String>>(api_url: A, base_url: B) -> Self {
        Self {
            name: "ThePirateBay".into(),
            api_url: api_url.into(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait::async_trait]
impl Indexer for IndexerThePirateBay {
    async fn search(&self, query: &str) -> IndexerResult {
        tracing::debug!("{} searching {query:?}", self.name);
        search::execute(&self.api_url, &self.base_url, query, 0).await
    }

    async fn feed(&self, category: Category) -> IndexerResult {
        tracing::debug!("{} fetching feed for {category:?}", self.name);
        let cats: &[u16] = match category {
            Category::Audio | Category::Music => &MUSIC_CATEGORIES,
            Category::Movie => &MOVIE_CATEGORIES,
            Category::Tv => &TVSHOW_CATEGORIES,
            Category::Book => &BOOK_CATEGORIES,
        };
        feed::execute(&self.api_url, &self.base_url, cats).await
    }
}
