use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use manteau_indexer_manager::IndexerManager;
use quick_xml::events::BytesText;
use std::borrow::Cow;

fn deserialize_category<'de, D>(deserializer: D) -> Result<Category, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CategoryVisitor;

    impl<'de> serde::de::Visitor<'de> for CategoryVisitor {
        type Value = Category;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("3000, 5000 or 7000 are the expected values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let parsed = v.parse::<u32>().map_err(|err| {
                serde::de::Error::custom(format!("expected a number, received {:?}: {:?}", v, err))
            })?;
            self.visit_u32(parsed)
        }

        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Category::try_from(value).map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_str(CategoryVisitor)
}

pub struct ApplicationRssXml(Cow<'static, str>);

impl IntoResponse for ApplicationRssXml {
    fn into_response(self) -> axum::response::Response {
        (
            [(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/rss+xml; charset=utf-8"),
            )],
            self.0,
        )
            .into_response()
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum QueryParams {
    Caps,
    Music,
    Search {
        #[serde(deserialize_with = "deserialize_category")]
        cat: Category,
        #[serde(default = "String::new")]
        q: String,
    },
}

impl QueryParams {
    async fn handle(&self, indexer: IndexerManager) -> ApplicationRssXml {
        match self {
            Self::Caps => handle_caps(),
            Self::Music => handle_feed(indexer, Category::Music).await,
            Self::Search { cat, q } => {
                if q.is_empty() {
                    handle_feed(indexer, *cat).await
                } else {
                    handle_search(indexer, *cat, q).await
                }
            }
        }
    }
}

fn handle_caps() -> ApplicationRssXml {
    ApplicationRssXml(Cow::Borrowed(
        crate::entity::torznab::capabilities::CAPABILITIES.as_str(),
    ))
}

fn write_item(
    writer: &mut quick_xml::writer::Writer<Vec<u8>>,
    item: IndexerEntry,
    category: Category,
) -> quick_xml::Result<()> {
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
            .write_text_content(BytesText::new(&item.date.to_rfc2822()))?;
        w.create_element("size")
            .write_text_content(BytesText::new(&item.size.as_u64().to_string()))?;
        w.create_element("description").write_empty()?;
        w.create_element("category")
            .write_text_content(BytesText::new(&category.kind().to_string()))?;
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
            .with_attribute(("value", category.kind().to_string().as_str()))
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

fn write_response(category: Category, entries: Vec<IndexerEntry>) -> ApplicationRssXml {
    let mut writer = quick_xml::writer::Writer::new(Vec::new());
    writer
        .create_element("rss")
        .with_attribute(("version", "2.0"))
        .with_attribute(("xmlns:atom", "http://www.w3.org/2005/Atom"))
        .with_attribute(("xmlns:torznab", "http://torznab.com/schemas/2015/feed"))
        .write_inner_content(|w| {
            w.create_element("channel").write_inner_content(|w| {
                w.create_element("atom:link")
                    .with_attribute(("href", "http://manteau:3000/"))
                    .with_attribute(("rel", "self"))
                    .with_attribute(("type", "application/rss+xml"))
                    .write_empty()?;
                w.create_element("title")
                    .write_text_content(BytesText::new("Manteau"))?;
                w.create_element("description")
                    .write_text_content(BytesText::new(
                        "Manteau is an aggregator for torrent search engines.",
                    ))?;
                w.create_element("link")
                    .write_text_content(BytesText::new("http://manteau:3000/"))?;
                w.create_element("language")
                    .write_text_content(BytesText::new("en-US"))?;
                w.create_element("category")
                    .write_text_content(BytesText::new("search"))?;

                for item in entries {
                    write_item(w, item, category)?;
                }

                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
    let inner = writer.into_inner();
    let result = String::from_utf8_lossy(&inner);
    let result = format!("{}{result}", crate::entity::torznab::DOM);
    ApplicationRssXml(Cow::Owned(result))
}

async fn handle_feed(indexer: IndexerManager, category: Category) -> ApplicationRssXml {
    let result = indexer.feed(category).await;
    if !result.errors.is_empty() {
        tracing::debug!("had the following errors: {:?}", result.errors);
    }
    write_response(category, result.entries)
}

async fn handle_search(
    indexer: IndexerManager,
    category: Category,
    query: &str,
) -> ApplicationRssXml {
    // TODO handle category in search
    let result = indexer.search(query).await;
    if !result.errors.is_empty() {
        tracing::debug!("had the following errors: {:?}", result.errors);
    }
    write_response(category, result.entries)
}

pub async fn handler(
    Extension(indexer): Extension<IndexerManager>,
    Query(params): Query<QueryParams>,
) -> ApplicationRssXml {
    tracing::debug!("GET /api/torznab params={params:?}");
    params.handle(indexer).await
}

#[cfg(test)]
mod tests {
    use super::{handler, QueryParams};
    use axum::extract::{Extension, Query};

    #[tokio::test]
    async fn success() {
        let res = handler(Extension(Default::default()), Query(QueryParams::Caps)).await;
        assert!(res.0.contains("manteau"));
    }
}

#[cfg(test)]
mod integration_tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use manteau_indexer_manager::prelude::{Category, IndexerResult};
    use tower::ServiceExt;

    #[derive(Debug, Clone, Default)]
    struct MockIndexer;

    #[async_trait::async_trait]
    impl manteau_indexer_manager::prelude::Indexer for MockIndexer {
        async fn search(&self, _query: &str) -> IndexerResult {
            IndexerResult::default()
        }
        async fn feed(&self, _category: Category) -> IndexerResult {
            IndexerResult::default()
        }
    }

    #[tokio::test]
    async fn caps_should_return_valid_xml() {
        crate::init_logs();

        let indexer = manteau_indexer_manager::IndexerManager::with_indexer(MockIndexer::default());
        let app = crate::router(indexer);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/torznab?t=caps")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(body.contains("manteau"));
    }

    #[tokio::test]
    async fn music_should_return_valid_xml() {
        crate::init_logs();

        let indexer = manteau_indexer_manager::IndexerManager::with_indexer(MockIndexer::default());
        let app = crate::router(indexer);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/torznab?t=music")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(body.contains("manteau"));
    }

    #[tokio::test]
    async fn search_music_should_return_valid_xml() {
        crate::init_logs();

        let indexer = manteau_indexer_manager::IndexerManager::with_indexer(MockIndexer::default());
        let app = crate::router(indexer);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/torznab?t=search&cat=3000&q=foo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(body.contains("manteau"));
    }
}
