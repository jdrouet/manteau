use chrono::{DateTime, Utc};
use manteau_indexer_helper::numeric::Number;
use manteau_indexer_prelude::{IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult};
use once_cell::sync::Lazy;
use scraper::{ElementRef, Html, Selector};

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
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryNameNotFound))
}

fn parse_link<'a>(elt: &'a ElementRef) -> Result<(String, &'a str), IndexerError> {
    let link = parse_link_element(elt)?;
    let name = link.text().collect::<String>();
    let name = sanitize_name(&name);
    let path = link
        .value()
        .attr("href")
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryLinkNotFound))?;
    Ok((name, path))
}

fn parse_size(elt: &ElementRef) -> Result<bytesize::ByteSize, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_SIZE_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryLinkNotFound))?;
    let value = value.text().collect::<String>();
    value.trim().parse::<bytesize::ByteSize>().map_err(|cause| {
        IndexerError::new(super::NAME, IndexerErrorReason::EntrySizeInvalid { cause })
    })
}

fn parse_seeders(elt: &ElementRef) -> Result<usize, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_SEEDER_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntrySeedersNotFound))?;
    let value = value.text().collect::<String>();
    value
        .trim()
        .parse::<Number>()
        .map(|num| num.as_value())
        .map_err(|cause| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntrySeedersInvalid { cause },
            )
        })
}

fn parse_leechers(elt: &ElementRef) -> Result<usize, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_LEECHER_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryLeechersNotFound))?;
    let value = value.text().collect::<String>();
    value
        .trim()
        .parse::<Number>()
        .map(|num| num.as_value())
        .map_err(|cause| {
            IndexerError::new(
                super::NAME,
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
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryDateNotFound))?;
    crate::date::parse(&value).map_err(|cause| {
        IndexerError::new(super::NAME, IndexerErrorReason::EntryDateInvalid { cause })
    })
}

fn parse_magnet(elt: &ElementRef) -> Result<String, IndexerError> {
    let value = elt
        .select(&SEARCH_ROW_MAGNET_SELECTOR)
        .next()
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryMagnetNotFound))?;
    value
        .value()
        .attr("href")
        .map(String::from)
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryMagnetNotFound))
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
        origin: super::NAME,
    })
}

pub fn parse(base_url: &str, html: &str) -> IndexerResult {
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
