use super::prelude::{
    Category, Indexer, IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult,
};
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

async fn fetch_page(base_url: &str, path: &str) -> Result<String, IndexerError> {
    let url = format!("{base_url}{path}");
    let req = reqwest::get(&url).await.map_err(|cause| {
        IndexerError::new(
            INDEXER_NAME,
            IndexerErrorReason::UnableToQuery {
                url: url.clone(),
                cause: cause.to_string(),
            },
        )
    })?;
    req.text().await.map_err(|err| {
        IndexerError::new(
            INDEXER_NAME,
            IndexerErrorReason::UnableToRead {
                url,
                cause: err.to_string(),
            },
        )
    })
}

async fn fetch_magnet(base_url: &str, path: &str) -> Result<String, IndexerError> {
    let html = fetch_page(base_url, path).await?;
    let html = Html::parse_document(html.as_str());

    html.select(&RESULT_LINK)
        .filter_map(|link| link.value().attr("href"))
        .filter(|link| link.starts_with("magnet:?"))
        .map(String::from)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryMagnetNotFound))
}

fn parse_link_element<'a>(elt: &'a ElementRef) -> Result<ElementRef<'a>, IndexerError> {
    elt.select(&NAME_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLinkNotFound))
}

fn parse_link<'a>(elt: &'a ElementRef) -> Result<(String, &'a str), IndexerError> {
    let link = parse_link_element(&elt)?;
    let name = link.text().collect::<String>().trim().to_string();
    let path = link
        .value()
        .attr("href")
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLinkNotFound))?;
    Ok((name, path))
}

fn parse_seeders(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt
        .select(&SEEDS_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySeedersNotFound))?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|cause| {
            IndexerError::new(
                INDEXER_NAME,
                IndexerErrorReason::EntrySeedersInvalid { cause },
            )
        })
}

fn parse_leechers(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt.select(&LEECHERS_SELECTOR).next().ok_or_else(|| {
        IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntryLeechersNotFound)
    })?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|cause| {
            IndexerError::new(
                INDEXER_NAME,
                IndexerErrorReason::EntryLeechersInvalid { cause },
            )
        })
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, IndexerError> {
    let value = elt
        .select(&SIZE_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySizeNotFound))?;

    let value = value
        .text()
        .next()
        .ok_or_else(|| IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySizeNotFound))?;

    value.parse::<bytesize::ByteSize>().map_err(|cause| {
        IndexerError::new(INDEXER_NAME, IndexerErrorReason::EntrySizeInvalid { cause })
    })
}

fn parse_list_row<'a>(base_url: &str, elt: ElementRef<'a>) -> Result<IndexerEntry, IndexerError> {
    let (name, link) = parse_link(&elt)?;
    let seeders = parse_seeders(&elt)?;
    let leechers = parse_leechers(&elt)?;
    let size = parse_size(&elt)?;
    // let magnet = fetch_magnet(base_url, link).await?;

    Ok(IndexerEntry {
        name,
        url: format!("{base_url}{link}"),
        size,
        seeders,
        leechers,
        magnet: link.to_string(),
        origin: INDEXER_NAME,
    })
}

async fn resolve_magnet(
    base_url: &str,
    mut entry: IndexerEntry,
) -> Result<IndexerEntry, IndexerError> {
    entry.magnet = fetch_magnet(base_url, &entry.magnet).await?;
    Ok(entry)
}

async fn parse_list_page(base_url: &str, html: &str) -> IndexerResult {
    let mut results = IndexerResult::default();
    let mut entries = Vec::new();

    // avoid having send issues due to Html
    {
        let html = Html::parse_document(html);

        for elt in html.select(&ROW_SELECTOR) {
            match parse_list_row(base_url, elt) {
                Ok(found) => entries.push(found),
                Err(error) => results.errors.push(error),
            }
        }
    }

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
}

#[async_trait::async_trait]
impl Indexer for Indexer1337x {
    async fn search(&self, query: &str) -> IndexerResult {
        let query = urlencoding::encode(query);
        let path = format!("/search/{query}/1/");
        let html = match fetch_page(&self.base_url, path.as_str()).await {
            Ok(value) => value,
            Err(error) => return IndexerResult::from(error),
        };

        parse_list_page(&self.base_url, &html).await
    }

    async fn feed(&self, category: Category) -> IndexerResult {
        let path = match category {
            Category::Audio | Category::Music => "/cat/Music/1/",
            Category::Movie => "/cat/Movies/1/",
            Category::Tv => "/cat/TV/1/",
            Category::Book => "/cat/Other/1/",
        };

        let html = match fetch_page(&self.base_url, path).await {
            Ok(value) => value,
            Err(error) => return IndexerResult::from(error),
        };

        parse_list_page(&self.base_url, &html).await
    }
}

#[cfg(test)]
mod tests {
    use super::Indexer1337x;
    use crate::prelude::Indexer;

    #[tokio::test]
    async fn basic_search() {
        let mut server = mockito::Server::new_async().await;
        let indexer = Indexer1337x::new(server.url().as_str());

        let search_page = server
            .mock("GET", "/search/how%20i%20met%20your%20mother/1/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!("../../asset/indexer-1337x-search-page.html"))
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
                include_str!("../../asset/indexer-1337x-result-page.html")
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
