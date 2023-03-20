use super::prelude::{Indexer, SearchResult, SearchResultError, SearchResultItem};
use once_cell::sync::Lazy;
use scraper::{ElementRef, Html, Selector};
use url::Url;

static ROW_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".card.search-result").unwrap());
static SEARCH_ROW_NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("h5.title a").unwrap());
static SEARCH_ROW_SIZE_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(2)").unwrap());
static SEARCH_ROW_SEEDER_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(3)").unwrap());
static SEARCH_ROW_LEECHER_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(4)").unwrap());
static SEARCH_ROW_MAGNET_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("a.dl-magnet").unwrap());

const BASE_URL: &str = "https://bitsearch.to";
const INDEXER_NAME: &str = "bitsearch";

fn sanitize_name(input: &str) -> String {
    input
        .split(&['\n', ' ', '\t'])
        .filter(|sec| !sec.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_link_element<'a>(elt: &'a ElementRef) -> Result<ElementRef<'a>, SearchResultError> {
    elt.select(&SEARCH_ROW_NAME_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find item name".into(),
            cause: None,
        })
}

fn parse_link<'a>(elt: &'a ElementRef) -> Result<(String, &'a str), SearchResultError> {
    let link = parse_link_element(&elt)?;
    let name = link.text().collect::<String>();
    let name = sanitize_name(&name);
    let path = link.value().attr("href").ok_or_else(|| SearchResultError {
        origin: INDEXER_NAME,
        message: "unable to find item link".into(),
        cause: None,
    })?;
    Ok((name, path))
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, SearchResultError> {
    let value = elt
        .select(&SEARCH_ROW_SIZE_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find item size".into(),
            cause: None,
        })?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<bytesize::ByteSize>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to parse size: {err}"),
            cause: None,
        })
}

fn parse_seeders(elt: &ElementRef) -> Result<u32, SearchResultError> {
    let value = elt
        .select(&SEARCH_ROW_SEEDER_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find item size".into(),
            cause: None,
        })?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<u32>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to parse seeders".into(),
            cause: Some(Box::new(err)),
        })
}

fn parse_leechers(elt: &ElementRef) -> Result<u32, SearchResultError> {
    let value = elt
        .select(&SEARCH_ROW_LEECHER_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find item size".into(),
            cause: None,
        })?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<u32>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to parse leechers".into(),
            cause: Some(Box::new(err)),
        })
}

fn parse_magnet(elt: &ElementRef) -> Result<String, SearchResultError> {
    let value = elt
        .select(&SEARCH_ROW_MAGNET_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find magnet".into(),
            cause: None,
        })?;
    value
        .value()
        .attr("href")
        .map(String::from)
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to read magnet".into(),
            cause: None,
        })
}

fn parse_row_result(
    base_url: &str,
    elt: ElementRef,
) -> Result<SearchResultItem, SearchResultError> {
    let (name, path) = parse_link(&elt)?;
    let size = parse_size(&elt)?;
    let seeders = parse_seeders(&elt)?;
    let leechers = parse_leechers(&elt)?;
    let magnet = parse_magnet(&elt)?;

    Ok(SearchResultItem {
        name,
        url: format!("{base_url}{path}"),
        size,
        seeders,
        leechers,
        magnet,
        origin: INDEXER_NAME,
    })
}

fn parse_search_result(base_url: &str, html: &str) -> SearchResult {
    let mut results = SearchResult::default();

    let html = Html::parse_document(html);

    for element in html.select(&ROW_SELECTOR) {
        match parse_row_result(base_url, element) {
            Ok(found) => results.entries.push(found),
            Err(error) => results.errors.push(error),
        };
    }

    results
}

#[derive(Debug)]
pub struct IndexerBitsearch {
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
            base_url: base_url.into(),
        }
    }

    async fn fetch_page(&self, url: Url) -> Result<String, SearchResultError> {
        let url_str = url.to_string();
        let req = reqwest::get(url).await.map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to query {url_str:?}"),
            cause: Some(Box::new(err)),
        })?;
        req.text().await.map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to read result from {url_str:?}"),
            cause: Some(Box::new(err)),
        })
    }
}

#[async_trait::async_trait(?Send)]
impl Indexer for IndexerBitsearch {
    fn name(&self) -> &'static str {
        INDEXER_NAME
    }

    async fn search(&self, query: &str) -> SearchResult {
        let url = format!("{}/search", self.base_url);
        let url = match Url::parse_with_params(&url, &[("q", query)]) {
            Ok(value) => value,
            Err(error) => {
                return SearchResult::from(SearchResultError {
                    origin: self.name(),
                    message: "unable to build search url".to_string(),
                    cause: Some(Box::new(error)),
                });
            }
        };

        let html = match self.fetch_page(url).await {
            Ok(value) => value,
            Err(error) => return SearchResult::from(error),
        };

        parse_search_result(&self.base_url, html.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::IndexerBitsearch;
    use crate::indexer::prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        crate::init_logs();

        let mut server = mockito::Server::new_async().await;
        let indexer = IndexerBitsearch::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search?q=how+i+met+your+mother")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!(
                "../../../asset/indexer-bitsearch-search-page.html"
            ))
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
