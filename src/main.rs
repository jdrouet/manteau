mod indexer;

#[tokio::main]
async fn main() {
    let manager = indexer::IndexerManager::default();
    let results = manager.search("how i met your mother").await;
    println!("results: {:#?}", results.entries);
    println!("errors: {:#?}", results.errors);
}

#[cfg(test)]
fn init_logs() {
    if let Err(_) = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
    {
        // NOTHING TO DO
    }
}
