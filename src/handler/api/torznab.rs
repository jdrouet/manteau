use axum::extract::Query;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "t")]
pub enum QueryParams {
    Caps,
}

pub async fn handler(Query(params): Query<QueryParams>) -> &'static str {
    println!("GET /api/torznab params={params:?}");
    "Hello, World!"
}
