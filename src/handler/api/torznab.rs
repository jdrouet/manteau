use axum::extract::Query;
use std::borrow::Cow;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum QueryParams {
    Caps,
}

impl QueryParams {
    fn handle(&self) -> Cow<'static, str> {
        match self {
            Self::Caps => Cow::Borrowed(include_str!("./capabilities.xml")),
        }
    }
}

pub async fn handler(Query(params): Query<QueryParams>) -> Cow<'static, str> {
    tracing::debug!("GET /api/torznab params={params:?}");
    params.handle()
}
