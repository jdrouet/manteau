use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use manteau_indexer_manager::IndexerManager;
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

fn write_item(result: &mut String, item: IndexerEntry, category: Category) {
    result.push_str("<item>");
    result.push_str(&format!("<title>{}</title>", item.name));
    result.push_str(&format!("<guid>{}</guid>", item.url));
    result.push_str("<type>public</type>");
    result.push_str(&format!("<comments>{}</comments>", item.url));
    result.push_str(&format!("<size>{}</size>", item.size.as_u64()));
    result.push_str("<description />");
    result.push_str(&format!("<category>{}</category>", category.kind()));
    result.push_str(r#"<torznab:attr name="genre" value="" />"#);
    result.push_str(r#"<torznab:attr name="downloadvolumefactor" value="0" />"#);
    result.push_str(r#"<torznab:attr name="uploadvolumefactor" value="1" />"#);
    result.push_str(&format!(
        "<torznab:attr name=\"magneturl\" value={:?} />",
        item.magnet
    ));
    result.push_str(&format!(
        "<torznab:attr name=\"category\" value=\"{}\" />",
        category.kind()
    ));
    result.push_str(&format!(
        "<torznab:attr name=\"seeders\" value=\"{}\" />",
        item.seeders
    ));
    result.push_str(&format!(
        "<torznab:attr name=\"peers\" value=\"{}\" />",
        item.leechers
    ));
    result.push_str("</item>");
}

fn write_response(category: Category, entries: Vec<IndexerEntry>) -> ApplicationRssXml {
    let mut result = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    result.push_str(r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:torznab="http://torznab.com/schemas/2015/feed">"#);
    result.push_str("<channel>");
    result.push_str(
        r#"<atom:link href="http://manteau:3000/" rel="self" type="application/rss+xml" />"#,
    );
    result.push_str(r#"<title>Manteau</title>"#);
    result.push_str(
        r#"<description>Manteau is an aggregator for torrent search engines.</description>"#,
    );
    result.push_str(r#"<link>http://manteau:3000/</link>"#);
    result.push_str(r#"<language>en-US</language>"#);
    result.push_str(r#"<category>search</category>"#);
    for item in entries {
        write_item(&mut result, item, category);
    }
    result.push_str("</channel>");
    result.push_str("</rss>");
    ApplicationRssXml(Cow::Owned(result))
}

async fn handle_feed(indexer: IndexerManager, category: Category) -> ApplicationRssXml {
    let entries = indexer.feed(category).await.entries;
    write_response(category, entries)
}

async fn handle_search(
    indexer: IndexerManager,
    category: Category,
    query: &str,
) -> ApplicationRssXml {
    // TODO handle category in search
    let entries = indexer.search(query).await.entries;
    write_response(category, entries)
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
