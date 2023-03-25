use manteau_indexer_prelude::{
    Category, Indexer, IndexerBuilder, IndexerError, IndexerErrorReason, IndexerResult,
};
use reqwest::IntoUrl;
use url::Url;

mod date;
mod search;

const BASE_URL: &str = "https://bitsearch.to";
pub const NAME: &str = "bitsearch";

async fn fetch_page<U: IntoUrl + ToString>(url: U) -> Result<String, IndexerError> {
    let url_str = url.to_string();
    let req = reqwest::get(url).await.map_err(|err| {
        IndexerError::new(
            NAME,
            IndexerErrorReason::UnableToQuery {
                url: url_str.clone(),
                cause: err.to_string(),
            },
        )
    })?;
    req.text().await.map_err(|err| {
        IndexerError::new(
            NAME,
            IndexerErrorReason::UnableToRead {
                url: url_str,
                cause: err.to_string(),
            },
        )
    })
}

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
        tracing::info!("building bitsearch indexer named {name:?}");
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
        let url = format!("{}/search", self.base_url);
        let url = match Url::parse_with_params(&url, &[("q", query)]) {
            Ok(value) => value,
            Err(cause) => {
                return IndexerResult::from(IndexerError::new(
                    NAME,
                    IndexerErrorReason::UnableToBuildUrl { cause },
                ));
            }
        };

        let html = match fetch_page(url).await {
            Ok(value) => value,
            Err(error) => return IndexerResult::from(error),
        };

        search::parse(&self.base_url, html.as_str())
    }

    async fn feed(&self, category: Category) -> IndexerResult {
        tracing::debug!("{} fetching feed for {category:?}", self.name);
        let path = match category {
            Category::Audio | Category::Music => "/music",
            Category::Movie => "/libraries",
            Category::Tv => "/libraries?type=tvSeries",
            _ => return IndexerResult::default(),
        };
        let url = format!("{}{path}", self.base_url);

        let html = match fetch_page(url).await {
            Ok(value) => value,
            Err(error) => return IndexerResult::from(error),
        };

        search::parse(&self.base_url, html.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::IndexerBitsearch;
    use manteau_indexer_prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        let mut server = mockito::Server::new_async().await;
        let indexer = IndexerBitsearch::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search?q=how+i+met+your+mother")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!("./search.html"))
            .create_async()
            .await;

        let results = indexer.search("how i met your mother").await;
        println!("results: {results:#?}");
        assert_eq!(results.entries.len(), 20);
        assert_eq!(results.errors.len(), 0);
        assert_eq!(results.entries[0].name, "How I Met Your Mother (2005) Season 1-9 S01-S09 (1080p MIXED x265 HEVC 10bit AAC 5.1 Silence)");
        assert_eq!(results.entries[0].seeders, 111);
        assert_eq!(results.entries[0].leechers, 608);
        assert_eq!(results.entries[0].size.to_string(), "104.0 GB");
        assert_eq!(results.entries[1].name, "How I Met Your Mother Season 1");

        search_page.assert_async().await;
    }
}
