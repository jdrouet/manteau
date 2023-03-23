use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use quick_xml::events::BytesText;
use quick_xml::writer::Writer;
use quick_xml::Result;
use std::borrow::Cow;

const DOM: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

#[derive(Debug, serde::Deserialize)]
pub struct TorznabConfig {
    #[serde(default = "TorznabConfig::default_name")]
    pub name: Cow<'static, str>,
    #[serde(default = "TorznabConfig::default_description")]
    pub description: Cow<'static, str>,
    #[serde(default = "TorznabConfig::default_base_url")]
    pub base_url: String,
}

impl Default for TorznabConfig {
    fn default() -> Self {
        Self {
            name: Self::default_name(),
            description: Self::default_description(),
            base_url: Self::default_base_url(),
        }
    }
}

impl TorznabConfig {
    fn default_name() -> Cow<'static, str> {
        Cow::Borrowed(env!("CARGO_PKG_NAME"))
    }

    fn default_description() -> Cow<'static, str> {
        Cow::Borrowed("Manteau is an aggregator for torrent search engines.")
    }

    fn default_base_url() -> String {
        if let Ok(base_url) = std::env::var("BASE_URL") {
            base_url
        } else {
            let host = std::env::var("HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
            let port = std::env::var("PORT").unwrap_or_else(|_| String::from("3000"));
            format!("http://{host}:{port}")
        }
    }

    pub fn build(self) -> TorznabBuilder {
        TorznabBuilder {
            name: self.name,
            description: self.description,
            base_url: self.base_url,
        }
    }
}

pub struct TorznabBuilder {
    name: Cow<'static, str>,
    description: Cow<'static, str>,
    base_url: String,
}

#[cfg(test)]
impl Default for TorznabBuilder {
    fn default() -> Self {
        TorznabConfig::default().build()
    }
}

// Capabilities
impl TorznabBuilder {
    pub fn capabilities(&self) -> String {
        let mut writer = Writer::new(Vec::new());
        self.write_caps(&mut writer)
            .expect("build capabilities xml");
        let inner = writer.into_inner();
        let result = String::from_utf8_lossy(&inner);
        format!("{}{result}", DOM)
    }

    fn write_caps(&self, writer: &mut Writer<Vec<u8>>) -> Result<()> {
        writer.create_element("caps").write_inner_content(|w| {
            self.write_server(w)?;
            self.write_limits(w)?;
            self.write_searching(w)?;
            self.write_categories(w)?;
            Ok(())
        })?;
        Ok(())
    }

    fn write_server(&self, writer: &mut Writer<Vec<u8>>) -> Result<()> {
        writer
            .create_element("server")
            .write_text_content(BytesText::new(&self.name))?;
        Ok(())
    }

    fn write_limits(&self, writer: &mut Writer<Vec<u8>>) -> Result<()> {
        writer
            .create_element("limits")
            .with_attribute(("default", "100"))
            .with_attribute(("max", "100"))
            .write_empty()?;
        Ok(())
    }

    fn write_searching(&self, writer: &mut Writer<Vec<u8>>) -> Result<()> {
        writer
            .create_element("searching")
            .write_inner_content(|w| {
                w.create_element("search")
                    .with_attribute(("available", "yes"))
                    .with_attribute(("supportedParams", "q"))
                    .write_empty()?;
                w.create_element("tv-search")
                    .with_attribute(("available", "yes"))
                    .with_attribute(("supportedParams", "q,season,ep"))
                    .write_empty()?;
                w.create_element("movie-search")
                    .with_attribute(("available", "yes"))
                    .with_attribute(("supportedParams", "q"))
                    .write_empty()?;
                w.create_element("music-search")
                    .with_attribute(("available", "yes"))
                    .with_attribute(("supportedParams", "q"))
                    .write_empty()?;
                w.create_element("book-search")
                    .with_attribute(("available", "yes"))
                    .with_attribute(("supportedParams", "q"))
                    .write_empty()?;
                Ok(())
            })?;
        Ok(())
    }

    fn write_categories(&self, writer: &mut Writer<Vec<u8>>) -> Result<()> {
        writer
            .create_element("categories")
            .write_inner_content(|w| {
                w.create_element("category")
                    .with_attribute(("id", "2000"))
                    .with_attribute(("name", "Movies"))
                    .write_empty()?;
                w.create_element("category")
                    .with_attribute(("id", "3000"))
                    .with_attribute(("name", "Audio"))
                    .write_empty()?;
                w.create_element("category")
                    .with_attribute(("id", "5000"))
                    .with_attribute(("name", "TV"))
                    .write_empty()?;
                w.create_element("category")
                    .with_attribute(("id", "7000"))
                    .with_attribute(("name", "Books"))
                    .write_empty()?;
                Ok(())
            })?;
        Ok(())
    }
}

// Feed
impl TorznabBuilder {
    pub fn feed(&self, category: Category, entries: &[IndexerEntry]) -> String {
        let mut writer = Writer::new(Vec::new());
        self.write_rss(&mut writer, category, entries)
            .expect("build feed");
        let inner = writer.into_inner();
        let result = String::from_utf8_lossy(&inner);
        format!("{}{result}", DOM)
    }

    fn write_rss(
        &self,
        writer: &mut Writer<Vec<u8>>,
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
                        .with_attribute(("href", self.base_url.as_str()))
                        .with_attribute(("rel", "self"))
                        .with_attribute(("type", "application/rss+xml"))
                        .write_empty()?;
                    w.create_element("title")
                        .write_text_content(BytesText::new(&self.name))?;
                    w.create_element("description")
                        .write_text_content(BytesText::new(&self.description))?;
                    w.create_element("link")
                        .write_text_content(BytesText::new(self.base_url.as_str()))?;
                    w.create_element("language")
                        .write_text_content(BytesText::new("en-US"))?;
                    w.create_element("category")
                        .write_text_content(BytesText::new("search"))?;

                    for item in entries {
                        self.write_item(w, category, item)?;
                    }

                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        Ok(())
    }

    fn write_item(
        &self,
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
}
