use crate::prelude::{
    Category, Indexer, IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use reqwest::IntoUrl;
use scraper::{ElementRef, Html, Selector};
use url::Url;

mod date;

static ROW_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(".card.search-result").unwrap());
static SEARCH_ROW_NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("h5.title a").unwrap());
static SEARCH_ROW_SIZE_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(2)").unwrap());
static SEARCH_ROW_SEEDER_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(3)").unwrap());
static SEARCH_ROW_LEECHER_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats div:nth-child(4)").unwrap());
static SEARCH_ROW_DATE_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".stats img[alt=\"Date\"]").unwrap());
static SEARCH_ROW_MAGNET_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("a.dl-magnet").unwrap());

const BASE_URL: &str = "https://bitsearch.to";
const INDEXER_NAME: &str = "bitsearch";

async fn fetch_page<U: IntoUrl + ToString>(url: U) -> Result<String, IndexerError> {
    let url_str = url.to_string();
    let req = reqwest::get(url).await.map_err(|err| {
        IndexerError::new(
            INDEXER_NAME,
            IndexerErrorReason::UnableToQuery {
                url: url_str.clone(),
                cause: err.to_string(),
            },
        )
    })?;
    req.text().await.map_err(|err| {
        IndexerError::new(
            INDEXER_NAME,
            IndexerErrorReason::UnableToRead {
                url: url_str,
                cause: err.to_string(),
            },
        )
    })
}

fn sanitize_name(input: &str) -> String {
    input
        .split(&['\n', ' ', '\t'])
        .filter(|sec| !sec.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_link_element<'a>(elt: &'a ElementRef) -> Result<ElementRef<'a>, IndexerError> {
    elt.select(&SEARCH_ROW_NAME_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryNameNotFound))
}

fn parse_link<'a>(elt: &'a ElementRef) -> Result<(String, &'a str), IndexerError> {
    let link = parse_link_element(&elt)?;
    let name = link.text().collect::<String>();
    let name = sanitize_name(&name);
    let path = link
        .value()
        .attr("href")
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLinkNotFound))?;
    Ok((name, path))
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_SIZE_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLinkNotFound))?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<bytesize::ByteSize>()
        .map_err(|cause| {
            IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySizeInvalid { cause })
        })
}

fn parse_seeders(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_SEEDER_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySeedersNotFound))?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<u32>()
        .map_err(|cause| {
            IndexerError::new(
                INDEXER_NAME,
                IndexerErrorReason::EntrySeedersInvalid { cause },
            )
        })
}

fn parse_leechers(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_LEECHER_SELECTOR)
        .next()
        .ok_or_else(|| {
            IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLeechersNotFound)
        })?;
    value
        .text()
        .collect::<String>()
        .trim()
        .parse::<u32>()
        .map_err(|cause| {
            IndexerError::new(
                INDEXER_NAME,
                IndexerErrorReason::EntryLeechersInvalid { cause },
            )
        })
}

fn parse_date(elt: &ElementRef) -> Result<DateTime<Utc>, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_DATE_SELECTOR)
        .next()
        .and_then(|child| child.parent())
        .and_then(|child| child.children().find_map(|c| c.value().as_text()))
        .map(|text| text.to_string())
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryDateNotFound))?;
    date::parse(&value).map_err(|cause| {
        IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryDateInvalid { cause })
    })
}

fn parse_magnet(elt: &ElementRef) -> Result<String, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_MAGNET_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryMagnetNotFound))?;
    value
        .value()
        .attr("href")
        .map(String::from)
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryMagnetNotFound))
}

fn parse_row_result(base_url: &str, elt: ElementRef) -> Result<IndexerEntry, IndexerError> {
    let (name, path) = parse_link(&elt)?;
    let size = parse_size(&elt)?;
    let seeders = parse_seeders(&elt)?;
    let leechers = parse_leechers(&elt)?;
    let date = parse_date(&elt)?;
    let magnet = parse_magnet(&elt)?;

    Ok(IndexerEntry {
        name,
        url: format!("{base_url}{path}"),
        date,
        size,
        seeders,
        leechers,
        magnet,
        origin: INDEXER_NAME,
    })
}

fn parse_search_result(base_url: &str, html: &str) -> IndexerResult {
    let mut results = IndexerResult::default();

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
}

#[async_trait::async_trait]
impl Indexer for IndexerBitsearch {
    async fn search(&self, query: &str) -> IndexerResult {
        let url = format!("{}/search", self.base_url);
        let url = match Url::parse_with_params(&url, &[("q", query)]) {
            Ok(value) => value,
            Err(cause) => {
                return IndexerResult::from(IndexerError::new(
                    INDEXER_NAME,
                    IndexerErrorReason::UnableToBuildUrl { cause },
                ));
            }
        };

        let html = match fetch_page(url).await {
            Ok(value) => value,
            Err(error) => return IndexerResult::from(error),
        };

        parse_search_result(&self.base_url, html.as_str())
    }

    async fn feed(&self, category: Category) -> IndexerResult {
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

        parse_search_result(&self.base_url, html.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::IndexerBitsearch;
    use crate::prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        let mut server = mockito::Server::new_async().await;
        let indexer = IndexerBitsearch::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search?q=how+i+met+your+mother")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!(
                "../../asset/indexer-bitsearch-search-page.html"
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
