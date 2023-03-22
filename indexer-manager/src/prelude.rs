use std::num::ParseIntError;

use bytesize::ByteSize;
use url::ParseError;

#[derive(Clone, Copy, Debug)]
pub enum Category {
    Audio,
    Book,
    Movie,
    Music,
    Tv,
}

impl Category {
    pub fn kind(&self) -> u32 {
        match self {
            Self::Audio | Self::Music => 3000,
            Self::Movie => 2000,
            Self::Tv => 5000,
            Self::Book => 7000,
        }
    }
}

impl TryFrom<u32> for Category {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            2000 => Ok(Self::Movie),
            3000 => Ok(Self::Audio),
            5000 => Ok(Self::Tv),
            7000 => Ok(Self::Book),
            _ => Err(format!("invalid category {value}")),
        }
    }
}

#[async_trait::async_trait]
pub trait Indexer: std::fmt::Debug {
    async fn search(&self, query: &str) -> IndexerResult;
    async fn feed(&self, category: Category) -> IndexerResult;
}

#[derive(Clone, Debug, Default)]
pub struct IndexerResult {
    pub entries: Vec<IndexerEntry>,
    pub errors: Vec<IndexerError>,
}

impl From<IndexerError> for IndexerResult {
    fn from(value: IndexerError) -> Self {
        Self {
            errors: vec![value],
            ..Default::default()
        }
    }
}

impl IndexerResult {
    pub fn merge(mut self, other: IndexerResult) -> Self {
        self.entries.extend(other.entries);
        self.errors.extend(other.errors);
        self
    }
}

#[derive(Clone, Debug)]
pub struct IndexerError {
    pub origin: &'static str,
    pub reason: IndexerErrorReason,
}

#[derive(Clone, Debug)]
pub enum IndexerErrorReason {
    EntryNameNotFound,
    EntryLinkNotFound,
    EntrySizeNotFound,
    EntrySizeInvalid { cause: String },
    EntrySeedersNotFound,
    EntrySeedersInvalid { cause: ParseIntError },
    EntryLeechersNotFound,
    EntryLeechersInvalid { cause: ParseIntError },
    EntryMagnetNotFound,
    UnableToQuery { url: String, cause: String },
    UnableToRead { url: String, cause: String },
    UnableToBuildUrl { cause: ParseError },
}

impl IndexerError {
    pub fn new(origin: &'static str, reason: IndexerErrorReason) -> Self {
        Self { origin, reason }
    }
}

// impl std::fmt::Display for IndexerError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "SearchResultError {{ origin={:?}, message={:?} }}",
//             self.origin, self.message
//         )
//     }
// }

// impl std::error::Error for IndexerError {
//     fn description(&self) -> &str {
//         &self.message
//     }

//     fn cause(&self) -> Option<&dyn std::error::Error> {
//         self.cause
//             .as_ref()
//             .map(|v| v.as_ref() as &dyn std::error::Error)
//     }
// }

#[derive(Clone, Debug)]
pub struct IndexerEntry {
    pub name: String,
    pub url: String,
    pub size: ByteSize,
    pub seeders: u32,
    pub leechers: u32,
    pub magnet: String,
    pub origin: &'static str,
}