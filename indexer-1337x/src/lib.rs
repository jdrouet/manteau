use manteau_indexer_prelude::{
    Category, Indexer, IndexerBuilder, IndexerEntry, IndexerError, IndexerErrorReason,
    IndexerResult,
};

mod date;
mod search;
mod torrent;

const BASE_URL: &str = "https://1337x.to";
pub const NAME: &str = "1337x";

async fn fetch_page(base_url: &str, path: &str) -> Result<String, IndexerError> {
    let url = format!("{base_url}{path}");
    let req = reqwest::get(&url).await.map_err(|cause| {
        IndexerError::new(
            NAME,
            IndexerErrorReason::UnableToQuery {
                url: url.clone(),
                cause: cause.to_string(),
            },
        )
    })?;
    req.text().await.map_err(|err| {
        IndexerError::new(
            NAME,
            IndexerErrorReason::UnableToRead {
                url,
                cause: err.to_string(),
            },
        )
    })
}

async fn resolve_magnet(
    base_url: &str,
    mut entry: IndexerEntry,
) -> Result<IndexerEntry, IndexerError> {
    let html = fetch_page(base_url, &entry.magnet).await?;
    entry.magnet = torrent::parse_magnet(html.as_str())?;
    Ok(entry)
}

async fn search(base_url: &str, path: &str) -> IndexerResult {
    let html = match fetch_page(base_url, path).await {
        Ok(value) => value,
        Err(error) => return IndexerResult::from(error),
    };

    let IndexerResult { entries, errors } = search::parse(base_url, html.as_str());
    let mut results = IndexerResult::from(errors);

    let entries = futures::future::join_all(
        entries
            .into_iter()
            .map(|entry| resolve_magnet(base_url, entry)),
    )
    .await;

    for entry in entries {
        match entry {
            Ok(found) => results.entries.push(found),
            Err(error) => results.errors.push(error),
        };
    }

    results
}

#[derive(Debug, serde::Deserialize)]
pub struct Indexer1337xConfig {
    #[serde(default = "Indexer1337xConfig::default_base_url")]
    pub base_url: String,
}

impl Indexer1337xConfig {
    fn default_base_url() -> String {
        BASE_URL.into()
    }
}

impl IndexerBuilder for Indexer1337xConfig {
    fn build(self, name: String) -> Box<dyn Indexer + Send + Sync + 'static> {
        tracing::info!("building 1337x indexer named {name:?}");
        Box::new(Indexer1337x {
            name,
            base_url: self.base_url,
        })
    }
}

#[derive(Debug)]
pub struct Indexer1337x {
    name: String,
    base_url: String,
}

impl Default for Indexer1337x {
    fn default() -> Self {
        Self::new(BASE_URL)
    }
}

impl Indexer1337x {
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        Self {
            name: "1337x".into(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait::async_trait]
impl Indexer for Indexer1337x {
    async fn search(&self, query: &str) -> IndexerResult {
        tracing::debug!("{} searching {query:?}", self.name);
        let query = urlencoding::encode(query);
        let path = format!("/search/{query}/1/");

        search(self.base_url.as_str(), path.as_str()).await
    }

    async fn feed(&self, category: Category) -> IndexerResult {
        tracing::debug!("{} fetching feed for {category:?}", self.name);
        let path = match category {
            Category::Audio | Category::Music => "/cat/Music/1/",
            Category::Movie => "/cat/Movies/1/",
            Category::Tv => "/cat/TV/1/",
            Category::Book => "/cat/Other/1/",
        };

        search(self.base_url.as_str(), path).await
    }
}

#[cfg(test)]
mod tests {
    use super::Indexer1337x;
    use manteau_indexer_prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        let mut server = mockito::Server::new_async().await;
        let indexer = Indexer1337x::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search/how%20i%20met%20your%20mother/1/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!("./search.html"))
            .create_async()
            .await;

        let result_page = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/torrent/(\d+)/(.*)/$".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(
                include_str!("./torrent.html")
                    .replace("%TORRENT_NAME%", "How I Met Your Mother - Season 4"),
            )
            .expect(20)
            .create_async()
            .await;

        let results = indexer.search("how i met your mother").await;
        assert_eq!(results.entries.len(), 20);
        assert_eq!(results.errors.len(), 0);
        assert_eq!(results.entries[0].name, "How I Met Your Mother - Season 4");
        assert_eq!(results.entries[0].seeders, 26);
        assert_eq!(results.entries[0].leechers, 9);
        assert_eq!(results.entries[0].size.to_string(), "4.1 GB");
        assert_eq!(
            results.entries[1].name,
            "How I Met Your Mother S01-S09 COMPLETE DVDrip mixed"
        );
        assert_eq!(
            results.entries[2].name,
            "How I Met Your Mother Season 7 Complete HDTV Bzingaz"
        );
        assert_eq!(
            results.entries[3].name,
            "How I Met Your Mother [KiSS] - Season 1 Complete E-Subs WEB-Dl"
        );

        search_page.assert_async().await;
        result_page.assert_async().await;
    }
}
