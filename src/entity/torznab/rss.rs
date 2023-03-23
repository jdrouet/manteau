use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use quick_xml::events::BytesText;
use quick_xml::writer::Writer;
use quick_xml::Result;

pub fn build_feed(base_url: &str, category: Category, entries: &[IndexerEntry]) -> String {
    tracing::trace!("building rss feed for category {category:?}");
    let mut writer = Writer::new(Vec::new());
    write_rss(&mut writer, base_url, category, entries).expect("building rss feed");
    let inner = writer.into_inner();
    let result = String::from_utf8_lossy(&inner);
    format!("{}{result}", super::DOM)
}

fn write_rss(
    writer: &mut Writer<Vec<u8>>,
    base_url: &str,
    category: Category,
    entries: &[IndexerEntry],
) -> Result<()> {
    tracing::trace!(
        "writing rss for category {category:?} with {} entries",
        entries.len()
    );
    writer
        .create_element("rss")
        .with_attribute(("version", "2.0"))
        .with_attribute(("xmlns:atom", "http://www.w3.org/2005/Atom"))
        .with_attribute(("xmlns:torznab", "http://torznab.com/schemas/2015/feed"))
        .write_inner_content(|w| {
            w.create_element("channel").write_inner_content(|w| {
                w.create_element("atom:link")
                    .with_attribute(("href", base_url))
                    .with_attribute(("rel", "self"))
                    .with_attribute(("type", "application/rss+xml"))
                    .write_empty()?;
                w.create_element("title")
                    .write_text_content(BytesText::new(env!("CARGO_PKG_NAME")))?;
                w.create_element("description")
                    .write_text_content(BytesText::new(
                        "Manteau is an aggregator for torrent search engines.",
                    ))?;
                w.create_element("link")
                    .write_text_content(BytesText::new(base_url))?;
                w.create_element("language")
                    .write_text_content(BytesText::new("en-US"))?;
                w.create_element("category")
                    .write_text_content(BytesText::new("search"))?;

                for item in entries {
                    write_item(w, category, item)?;
                }

                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
    Ok(())
}

fn write_item(
    writer: &mut Writer<Vec<u8>>,
    category: Category,
    item: &IndexerEntry,
) -> quick_xml::Result<()> {
    tracing::trace!("writing item {:?}", item.name);
    writer.create_element("item").write_inner_content(|w| {
        w.create_element("title")
            .write_text_content(BytesText::new(&item.name))?;
        w.create_element("guid")
            .write_text_content(BytesText::new(&item.url))?;
        w.create_element("type")
            .write_text_content(BytesText::new("public"))?;
        w.create_element("comments")
            .write_text_content(BytesText::new(&item.url))?;
        w.create_element("pubDate")
            .write_text_content(BytesText::new(&item.date_str()))?;
        w.create_element("size")
            .write_text_content(BytesText::new(&item.size_str()))?;
        w.create_element("link")
            .write_text_content(BytesText::new(&item.magnet))?;
        w.create_element("enclosure")
            .with_attribute(("url", item.magnet.as_str()))
            .with_attribute(("length", item.size_str().as_str()))
            .with_attribute(("type", "application/x-bittorrent"))
            .write_empty()?;
        w.create_element("description").write_empty()?;
        w.create_element("category")
            .write_text_content(BytesText::new(category.kind_str()))?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "genre"))
            .with_attribute(("value", ""))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "downloadvolumefactor"))
            .with_attribute(("value", "0"))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "uploadvolumefactor"))
            .with_attribute(("value", "1"))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "magneturl"))
            .with_attribute(("value", item.magnet.as_str()))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "category"))
            .with_attribute(("value", category.kind_str()))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "seeders"))
            .with_attribute(("value", item.seeders.to_string().as_str()))
            .write_empty()?;
        w.create_element("torznab:attr")
            .with_attribute(("name", "peers"))
            .with_attribute(("value", item.leechers.to_string().as_str()))
            .write_empty()?;
        Ok(())
    })?;
    Ok(())
}
