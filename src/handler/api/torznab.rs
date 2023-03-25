use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use manteau_indexer_manager::IndexerManager;
use std::borrow::Cow;
use std::str::FromStr;

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
            Category::from_str(v).map_err(serde::de::Error::custom)
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

fn write_response(category: Category, entries: Vec<IndexerEntry>) -> ApplicationRssXml {
    let result = crate::entity::torznab::rss::build_feed("http://manteau:3000", category, &entries);
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
    use chrono::Utc;
    use manteau_indexer_manager::bytesize;
    use manteau_indexer_manager::prelude::{Category, IndexerEntry, IndexerResult};
    use manteau_indexer_manager::IndexerManager;
    use tower::ServiceExt;

    #[derive(Debug, Clone, Default)]
    struct MockIndexer {
        pub entries: Vec<IndexerEntry>,
    }

    #[async_trait::async_trait]
    impl manteau_indexer_manager::prelude::Indexer for MockIndexer {
        async fn search(&self, _query: &str) -> IndexerResult {
            IndexerResult::from(self.entries.clone())
        }
        async fn feed(&self, _category: Category) -> IndexerResult {
            IndexerResult::from(self.entries.clone())
        }
    }

    #[tokio::test]
    async fn caps_should_return_valid_xml() {
        crate::init_logs();

        let indexer = IndexerManager::with_indexer(MockIndexer::default());
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

        let mut mock = MockIndexer::default();
        mock.entries.push(IndexerEntry {
            name: "too".to_string(),
            url: "https://example.com".into(),
            date: Utc::now(),
            size: bytesize::ByteSize::mb(120),
            seeders: 10,
            leechers: 20,
            magnet: "magnet-url".into(),
            origin: "fake",
        });

        let indexer = IndexerManager::with_indexer(mock);
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

        let channel = rss::Channel::read_from(body.as_bytes()).unwrap();
        let items = channel.into_items();
        assert_eq!(items.len(), 1);
        for item in items {
            assert!(item.title().is_some());
            assert!(item.link().is_some());
        }
    }

    #[tokio::test]
    async fn search_music_should_return_valid_xml() {
        crate::init_logs();

        let indexer = IndexerManager::with_indexer(MockIndexer::default());
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

        let channel = rss::Channel::read_from(body.as_bytes()).unwrap();
        let items = channel.into_items();
        assert_eq!(items.len(), 0);
    }
}
