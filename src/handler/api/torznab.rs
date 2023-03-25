use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Extension;
use manteau_indexer_manager::prelude::{Category, IndexerEntry};
use manteau_indexer_manager::IndexerManager;
use std::borrow::Cow;

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
}

impl QueryParams {
    async fn handle(&self, indexer: IndexerManager) -> ApplicationRssXml {
        match self {
            Self::Caps => handle_caps(),
            Self::Music => handle_feed(indexer, Category::Music).await,
        }
    }
}

fn handle_caps() -> ApplicationRssXml {
    ApplicationRssXml(Cow::Borrowed(
        crate::entry::torznab::capabilities::CAPABILITIES.as_str(),
    ))
}

fn write_item(result: &mut String, item: IndexerEntry, category: Category) {
    result.push_str("<item>");
    result.push_str("<title>");
    result.push_str(item.name.as_str());
    result.push_str("</title>");
    result.push_str("<guid>");
    result.push_str(item.url.as_str());
    result.push_str("</guid>");
    result.push_str("<type>public</type>");
    result.push_str("<comments>");
    result.push_str(item.url.as_str());
    result.push_str("</comments>");
    result.push_str("<size>");
    result.push_str(&item.size.as_u64().to_string());
    result.push_str("</size>");
    result.push_str("<description />");
    result.push_str("<category>");
    result.push_str(&category.kind().to_string());
    result.push_str("</category>");
    result.push_str(r#"<torznab:attr name="genre" value="" />"#);
    result.push_str(r#"<torznab:attr name="downloadvolumefactor" value="0" />"#);
    result.push_str(r#"<torznab:attr name="uploadvolumefactor" value="1" />"#);
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

async fn handle_feed(indexer: IndexerManager, category: Category) -> ApplicationRssXml {
    let entries = indexer.feed(category).await.entries;

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
