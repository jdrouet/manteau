use bytesize::ByteSize;

#[async_trait::async_trait]
pub trait Indexer: std::fmt::Debug {
    fn name(&self) -> &'static str;
    async fn search(&self, query: &str) -> SearchResult;
}

#[derive(Debug, Default)]
pub struct SearchResult {
    pub entries: Vec<SearchResultItem>,
    pub errors: Vec<SearchResultError>,
}

impl From<SearchResultError> for SearchResult {
    fn from(value: SearchResultError) -> Self {
        Self {
            errors: vec![value],
            ..Default::default()
        }
    }
}

impl SearchResult {
    pub fn merge(mut self, other: SearchResult) -> Self {
        self.entries.extend(other.entries);
        self.errors.extend(other.errors);
        self
    }
}

#[derive(Debug)]
pub struct SearchResultError {
    pub origin: &'static str,
    pub message: String,
    pub cause: Option<Box<dyn std::error::Error + Sync + Send + 'static>>,
}

impl std::fmt::Display for SearchResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SearchResultError {{ origin={:?}, message={:?} }}",
            self.origin, self.message
        )
    }
}

impl std::error::Error for SearchResultError {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.cause
            .as_ref()
            .map(|v| v.as_ref() as &dyn std::error::Error)
    }
}

#[derive(Debug)]
pub struct SearchResultItem {
    pub name: String,
    pub url: String,
    pub size: ByteSize,
    pub seeders: u32,
    pub leechers: u32,
    pub magnet: String,
    pub origin: &'static str,
}
