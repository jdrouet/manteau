use axum::extract::Query;

pub async fn handler(Query(params): Query<std::collections::HashMap<String, String>>) {
    tracing::debug!("params={params:?}");
}
