use crate::service::torznab::TorznabBuilder;
use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use manteau_indexer_manager::IndexerManager;
use manteau_indexer_prelude::Category;
use std::str::FromStr;
use std::sync::Arc;

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

pub struct ApplicationRssXml(String);

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
    #[serde(rename = "tvsearch")]
    TvSearch {
        #[serde(deserialize_with = "deserialize_category")]
        cat: Category,
        q: Option<String>,
        #[serde(default)]
        season: Option<String>,
        #[serde(default)]
        ep: Option<String>,
    },
    Movie {
        #[serde(deserialize_with = "deserialize_category")]
        cat: Category,
        #[serde(default)]
        q: Option<String>,
    },
}

impl QueryParams {
    async fn handle(self, indexer: Arc<IndexerManager>, torznab: Arc<TorznabBuilder>) -> String {
        match self {
            Self::Caps => torznab.capabilities(),
            Self::Music => handle_feed(indexer, torznab, Category::Music).await,
            Self::Search { cat, q } => {
                if q.is_empty() {
                    handle_feed(indexer, torznab, cat).await
                } else {
                    handle_search(indexer, torznab, cat, q).await
                }
            }
            Self::TvSearch { cat, q, season, ep } => {
                if let Some(query) = q {
                    handle_tv_search(indexer, torznab, cat, query, season, ep).await
                } else {
                    handle_feed(indexer, torznab, cat).await
                }
            }
            Self::Movie { cat, q } => {
                if let Some(query) = q {
                    handle_search(indexer, torznab, cat, query).await
                } else {
                    handle_feed(indexer, torznab, cat).await
                }
            }
        }
    }
}

async fn handle_feed(
    indexer: Arc<IndexerManager>,
    torznab: Arc<TorznabBuilder>,
    category: Category,
) -> String {
    let result = indexer.feed(category).await;
    if !result.errors.is_empty() {
        tracing::debug!("had the following errors: {:?}", result.errors);
    }
    torznab.feed(category, &result.entries)
}

fn format_number(input: String) -> String {
    if input.len() == 1 {
        format!("0{input}")
    } else {
        input
    }
}

async fn handle_tv_search(
    indexer: Arc<IndexerManager>,
    torznab: Arc<TorznabBuilder>,
    category: Category,
    query: String,
    season: Option<String>,
    episode: Option<String>,
) -> String {
    // TODO handle category in search
    let query = match (season, episode) {
        (Some(s), Some(e)) => format!("{query} S{}E{}", format_number(s), format_number(e)),
        (Some(s), None) => format!("{query} S{}", format_number(s)),
        _ => query,
    };
    let result = indexer.search(&query).await;
    if !result.errors.is_empty() {
        tracing::debug!("had the following errors: {:?}", result.errors);
    }
    torznab.feed(category, &result.entries)
}

async fn handle_search(
    indexer: Arc<IndexerManager>,
    torznab: Arc<TorznabBuilder>,
    category: Category,
    query: String,
) -> String {
    // TODO handle category in search
    let result = indexer.search(query.as_str()).await;
    if !result.errors.is_empty() {
        tracing::debug!("had the following errors: {:?}", result.errors);
    }
    torznab.feed(category, &result.entries)
}

pub async fn handler(
    Extension(indexer): Extension<Arc<IndexerManager>>,
    Extension(torznab): Extension<Arc<TorznabBuilder>>,
    Query(params): Query<QueryParams>,
) -> ApplicationRssXml {
    tracing::debug!("GET /api/torznab params={params:?}");
    ApplicationRssXml(params.handle(indexer, torznab).await)
}

#[cfg(test)]
mod tests {
    use super::{handler, QueryParams};
    use axum::extract::{Extension, Query};

    #[tokio::test]
    async fn success() {
        let res = handler(
            Extension(Default::default()),
            Extension(Default::default()),
            Query(QueryParams::Caps),
        )
        .await;
        assert!(res.0.contains("manteau"));
    }
}

#[cfg(test)]
mod integration_tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use manteau_indexer_manager::IndexerManager;
    use manteau_indexer_prelude::bytesize;
    use manteau_indexer_prelude::{Category, IndexerEntry, IndexerResult};
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::service::torznab::TorznabBuilder;

    #[derive(Debug, Clone, Default)]
    struct MockIndexer {
        pub entries: Vec<IndexerEntry>,
    }

    #[async_trait::async_trait]
    impl manteau_indexer_prelude::Indexer for MockIndexer {
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
        let torznab = TorznabBuilder::default();
        let app = crate::router(Arc::new(indexer), Arc::new(torznab));

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
        let torznab = TorznabBuilder::default();
        let app = crate::router(Arc::new(indexer), Arc::new(torznab));

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
        let torznab = TorznabBuilder::default();
        let app = crate::router(Arc::new(indexer), Arc::new(torznab));

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
