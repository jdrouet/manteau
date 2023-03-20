use axum::extract::Query;
use std::collections::HashMap;

pub async fn handler(Query(params): Query<HashMap<String, String>>) -> &'static str {
    println!("GET /api/torznab params={params:?}");
    "Hello, World!"
}
