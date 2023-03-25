use manteau_indexer_prelude::{IndexerError, IndexerErrorReason};
use once_cell::sync::Lazy;
use scraper::{Html, Selector};

static RESULT_LINK: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("main.container div.row div.page-content div.box-info.torrent-detail-page div.no-top-radius div ul li a").unwrap()
});

pub fn parse_magnet(html: &str) -> Result<String, IndexerError> {
    let html = Html::parse_document(html);

    html.select(&RESULT_LINK)
        .filter_map(|link| link.value().attr("href"))
        .filter(|link| link.starts_with("magnet:?"))
        .map(String::from)
        .next()
        .ok_or_else(|| IndexerError::new(super::NAME, IndexerErrorReason::EntryMagnetNotFound))
}
