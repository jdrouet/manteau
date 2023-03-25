use super::prelude::{Indexer, SearchResult, SearchResultError, SearchResultItem};
use once_cell::sync::Lazy;
use scraper::{ElementRef, Html, Selector};

const BASE_URL: &str = "https://1337x.to";
const INDEXER_NAME: &str = "1337x";

static ROW_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".table-list tbody tr").unwrap());
static NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.name a:nth-child(2)").unwrap());
static SEEDS_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.seeds").unwrap());
static LEECHERS_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.leeches").unwrap());
static SIZE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.size").unwrap());
static RESULT_LINK: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("main.container div.row div.page-content div.box-info.torrent-detail-page div.no-top-radius div ul li a").unwrap()
});

async fn fetch_page(base_url: &str, path: &str) -> Result<String, SearchResultError> {
    let url = format!("{base_url}{path}");
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

async fn fetch_magnet(base_url: &str, path: &str) -> Result<String, SearchResultError> {
    let html = fetch_page(base_url, path).await?;
    let html = Html::parse_document(html.as_str());

    html.select(&RESULT_LINK)
        .filter_map(|link| link.value().attr("href"))
        .filter(|link| link.starts_with("magnet:?"))
        .map(String::from)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to find magnet in {path:?}"),
            cause: None,
        })
}

fn parse_link_element<'a>(elt: &'a ElementRef) -> Result<ElementRef<'a>, SearchResultError> {
    elt.select(&NAME_SELECTOR)
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
    let path = link.value().attr("href").ok_or_else(|| SearchResultError {
        origin: INDEXER_NAME,
        message: "unable to find item link".into(),
        cause: None,
    })?;
    Ok((name, path))
}

fn parse_seeders(elt: &ElementRef) -> Result<u32, SearchResultError> {
    let value = elt
        .select(&SEEDS_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find seeders".into(),
            cause: None,
        })?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to parse seeders".into(),
            cause: Some(Box::new(err)),
        })
}

fn parse_leechers(elt: &ElementRef) -> Result<u32, SearchResultError> {
    let value = elt
        .select(&LEECHERS_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find leechers".into(),
            cause: None,
        })?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to parse leechers".into(),
            cause: Some(Box::new(err)),
        })
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, SearchResultError> {
    let value = elt
        .select(&SIZE_SELECTOR)
        .next()
        .ok_or_else(|| SearchResultError {
            origin: INDEXER_NAME,
            message: "unable to find size".into(),
            cause: None,
        })?;

    let value = value.text().next().ok_or_else(|| SearchResultError {
        origin: INDEXER_NAME,
        message: "unable to find size".into(),
        cause: None,
    })?;

    value
        .parse::<bytesize::ByteSize>()
        .map_err(|err| SearchResultError {
            origin: INDEXER_NAME,
            message: format!("unable to parse size: {}", err),
            cause: None,
        })
}

async fn parse_row_result<'a>(
    base_url: &str,
    elt: ElementRef<'a>,
) -> Result<SearchResultItem, SearchResultError> {
    let (name, link) = parse_link(&elt)?;
    let seeders = parse_seeders(&elt)?;
    let leechers = parse_leechers(&elt)?;
    let size = parse_size(&elt)?;
    let magnet = fetch_magnet(base_url, link).await?;

    Ok(SearchResultItem {
        name,
        url: format!("{base_url}{link}"),
        size,
        seeders,
        leechers,
        magnet,
        origin: INDEXER_NAME,
    })
}

async fn parse_search_page(base_url: &str, html: &str) -> SearchResult {
    let mut results = SearchResult::default();

    let html = Html::parse_document(html);

    let calls = html
        .select(&ROW_SELECTOR)
        .map(|elt| parse_row_result(base_url, elt));
    let items = futures::future::join_all(calls).await;

    for element in items {
        match element {
            Ok(found) => results.entries.push(found),
            Err(error) => results.errors.push(error),
        };
    }

    results
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

    async fn execute(&self, query: &str) -> SearchResult {
        let query = urlencoding::encode(query);
        let path = format!("/search/{query}/1/");
        let html = match fetch_page(&self.base_url, path.as_str()).await {
            Ok(value) => value,
            Err(error) => {
                return SearchResult::from(SearchResultError {
                    origin: self.name(),
                    message: "unable to fetch search page".to_string(),
                    cause: Some(Box::new(error)),
                });
            }
        };

        parse_search_page(&self.base_url, &html).await
    }
}

#[async_trait::async_trait(?Send)]
impl Indexer for Indexer1337x {
    fn name(&self) -> &'static str {
        INDEXER_NAME
    }

    async fn search(&self, query: &str) -> SearchResult {
        self.execute(query).await
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
