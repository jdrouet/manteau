use super::NAME;
use chrono::{DateTime, Utc};
use manteau_indexer_prelude::{IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult};
use once_cell::sync::Lazy;
use scraper::{ElementRef, Html, Selector};

static ROW_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".table-list tbody tr").unwrap());
static NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.name a:nth-child(2)").unwrap());
static SEEDS_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.seeds").unwrap());
static LEECHERS_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.leeches").unwrap());
static SIZE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.size").unwrap());
static DATE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("td.coll-date").unwrap());

fn parse_link_element<'a>(elt: &'a ElementRef) -> Result<ElementRef<'a>, IndexerError> {
    elt.select(&NAME_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntryLinkNotFound))
}

fn parse_link<'a>(elt: &'a ElementRef) -> Result<(String, &'a str), IndexerError> {
    let link = parse_link_element(elt)?;
    let name = link.text().collect::<String>().trim().to_string();
    let path = link
        .value()
        .attr("href")
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntryLinkNotFound))?;
    Ok((name, path))
}

fn parse_seeders(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt
        .select(&SEEDS_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntrySeedersNotFound))?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|cause| IndexerError::new(NAME, IndexerErrorReason::EntrySeedersInvalid { cause }))
}

fn parse_leechers(elt: &ElementRef) -> Result<u32, IndexerError> {
    let value = elt
        .select(&LEECHERS_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntryLeechersNotFound))?;
    value
        .text()
        .collect::<String>()
        .parse::<u32>()
        .map_err(|cause| {
            IndexerError::new(NAME, IndexerErrorReason::EntryLeechersInvalid { cause })
        })
}

fn parse_date(elt: &ElementRef) -> Result<DateTime<Utc>, IndexerError> {
    let value = elt
        .select(&DATE_SELECTOR)
        .next()
        .and_then(|t| t.text().next())
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntryDateNotFound))?;
    crate::date::parse(value)
        .map_err(|cause| IndexerError::new(NAME, IndexerErrorReason::EntryDateInvalid { cause }))
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, IndexerError> {
    let value = elt
        .select(&SIZE_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntrySizeNotFound))?;

    let value = value
        .text()
        .next()
        .ok_or_else(|| IndexerError::new(NAME, IndexerErrorReason::EntrySizeNotFound))?;

    value
        .parse::<bytesize::ByteSize>()
        .map_err(|cause| IndexerError::new(NAME, IndexerErrorReason::EntrySizeInvalid { cause }))
}

fn parse_list_row(base_url: &str, elt: ElementRef) -> Result<IndexerEntry, IndexerError> {
    let (name, link) = parse_link(&elt)?;
    let seeders = parse_seeders(&elt)?;
    let leechers = parse_leechers(&elt)?;
    let size = parse_size(&elt)?;
    let date = parse_date(&elt)?;

    Ok(IndexerEntry {
        name,
        url: format!("{base_url}{link}"),
        date,
        size,
        seeders,
        leechers,
        magnet: link.to_string(),
        origin: NAME,
    })
}

pub fn parse(base_url: &str, html: &str) -> IndexerResult {
    let mut results = IndexerResult::default();
    let html = Html::parse_document(html);

    for elt in html.select(&ROW_SELECTOR) {
        match parse_list_row(base_url, elt) {
            Ok(found) => results.entries.push(found),
            Err(error) => results.errors.push(error),
        }
    }

    results
}
