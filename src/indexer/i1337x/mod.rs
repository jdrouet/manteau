use super::prelude::{Indexer, SearchResult, SearchResultError, SearchResultItem};
use once_cell::sync::Lazy;
use scraper::{Html, Selector};

const BASE_URL: &str = "https://1337x.to";
const INDEXER_NAME: &str = "1337x";

static SEARCH_TAB_ROW_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".table-list tbody tr").unwrap());
static SEARCH_ROW_NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.name a:nth-child(2)").unwrap());
static SEARCH_ROW_SEEDS_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.seeds").unwrap());
static SEARCH_ROW_LEECHERS_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.leeches").unwrap());
static SEARCH_ROW_SIZE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.size").unwrap());
static RESULT_LINK: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("main.container div.row div.page-content div.box-info.torrent-detail-page div.no-top-radius div ul li a").unwrap()
});

struct ResultPage(Html);

impl From<&str> for ResultPage {
    fn from(value: &str) -> Self {
        Self(Html::parse_document(value))
    }
}

impl ResultPage {
    fn rows(self) -> Vec<ResultPageRow> {
        self.0
            .select(&SEARCH_TAB_ROW_SELECTOR)
            .filter_map(|tr| {
                let link = match tr.select(&SEARCH_ROW_NAME_SELECTOR).next() {
                    Some(value) => value,
                    None => {
                        tracing::debug!("unable to get name column");
                        return None;
                    }
                };
                let name: String = link.text().collect();
                let name = name.trim().to_string();
                let path = match link.value().attr("href") {
                    Some(value) => value.to_string(),
                    None => {
                        tracing::debug!("unable to get link for {name}");
                        return None;
                    }
                };
                let seeders = tr.select(&SEARCH_ROW_SEEDS_SELECTOR).next().and_then(|td| {
                    let inner: String = td.text().collect();
                    inner.parse::<u32>().ok()
                });
                let seeders = match seeders {
                    Some(value) => value,
                    None => {
                        tracing::debug!("unable to get seeders for {name}");
                        return None;
                    }
                };
                let leechers = tr
                    .select(&SEARCH_ROW_LEECHERS_SELECTOR)
                    .next()
                    .and_then(|td| {
                        let inner: String = td.text().collect();
                        inner.parse::<u32>().ok()
                    });
                let leechers = match leechers {
                    Some(value) => value,
                    None => {
                        tracing::debug!("unable to get leechers for {name}");
                        return None;
                    }
                };
                let size = tr.select(&SEARCH_ROW_SIZE_SELECTOR).next().map(|td| {
                    let inner: String = td.text().next().unwrap_or_default().to_string();
                    inner.parse::<bytesize::ByteSize>()
                });
                let size = match size {
                    Some(Ok(value)) => value,
                    Some(Err(inner)) => {
                        tracing::debug!("unable to parse size for {name}: {inner:?}");
                        return None;
                    }
                    None => {
                        tracing::debug!("unable to get leechers for {name}");
                        return None;
                    }
                };

                Some(ResultPageRow {
                    name,
                    path,
                    size,
                    seeders,
                    leechers,
                })
            })
            .collect()
    }
}

struct ResultPageRow {
    name: String,
    path: String,
    size: bytesize::ByteSize,
    seeders: u32,
    leechers: u32,
}

impl ResultPageRow {
    fn into_search_result(self, base_url: &str, magnet: String) -> SearchResultItem {
        SearchResultItem {
            name: self.name,
            url: format!("{base_url}{}", self.path),
            size: self.size,
            seeders: self.seeders,
            leechers: self.leechers,
            magnet,
            origin: INDEXER_NAME,
        }
    }
}

#[derive(Debug)]
pub struct Indexer1337x {
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
            base_url: base_url.into(),
        }
    }

    async fn fetch_page(&self, path: &str) -> Result<String, SearchResultError> {
        let url = format!("{}{path}", self.base_url);
        let req = reqwest::get(&url).await.map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to query {url:?}"),
            cause: Some(Box::new(err)),
        })?;
        req.text().await.map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to read result from {url:?}"),
            cause: Some(Box::new(err)),
        })
    }

    async fn fetch_result(
        &self,
        row: ResultPageRow,
    ) -> Result<SearchResultItem, SearchResultError> {
        let html = self.fetch_page(row.path.as_str()).await?;
        let html = Html::parse_document(html.as_str());

        let magnet = html
            .select(&RESULT_LINK)
            .filter_map(|link| link.value().attr("href"))
            .filter(|link| link.starts_with("magnet:?"))
            .map(String::from)
            .next();

        if let Some(link) = magnet {
            Ok(row.into_search_result(self.base_url.as_str(), link))
        } else {
            Err(SearchResultError {
                origin: self.name(),
                message: "unable to read magned href".into(),
                cause: None,
            })
        }
    }
}

#[async_trait::async_trait]
impl Indexer for Indexer1337x {
    fn name(&self) -> &'static str {
        INDEXER_NAME
    }

    async fn search(&self, query: &str) -> SearchResult {
        let mut result = SearchResult::default();

        let query = urlencoding::encode(query);
        let path = format!("/search/{query}/1/");
        let html = match self.fetch_page(path.as_str()).await {
            Ok(value) => value,
            Err(error) => {
                result.errors.push(SearchResultError {
                    origin: self.name(),
                    message: "unable to fetch search page".to_string(),
                    cause: Some(Box::new(error)),
                });
                return result;
            }
        };
        let rows = ResultPage::from(html.as_str()).rows();

        for row in rows {
            match self.fetch_result(row).await {
                Ok(item) => result.entries.push(item),
                Err(error) => result.errors.push(error),
            };
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Indexer1337x;
    use crate::indexer::prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        crate::init_logs();

        let mut server = mockito::Server::new_async().await;
        let indexer = Indexer1337x::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search/how%20i%20met%20your%20mother/1/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!(
                "../../../asset/indexer-1337x-search-page.html"
            ))
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
                include_str!("../../../asset/indexer-1337x-result-page.html")
                    .replace("%RESULT_NAME%", "How I Met Your Mother - Season 4"),
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
