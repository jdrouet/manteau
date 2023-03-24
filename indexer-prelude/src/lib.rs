pub use bytesize;

use bytesize::ByteSize;
use chrono::{DateTime, Utc};
use std::{num::ParseIntError, str::FromStr};
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

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Audio | Self::Music => "3000",
            Self::Movie => "2000",
            Self::Tv => "5000",
            Self::Book => "7000",
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

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2000" => Ok(Self::Movie),
            "3000" => Ok(Self::Audio),
            "5000" => Ok(Self::Tv),
            "7000" => Ok(Self::Book),
            _ => Err(format!("invalid category {s:?}")),
        }
    }
}

pub trait IndexerBuilder: std::fmt::Debug {
    fn build(self, name: String) -> Box<dyn Indexer + Send + Sync + 'static>;
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

impl From<Vec<IndexerEntry>> for IndexerResult {
    fn from(entries: Vec<IndexerEntry>) -> Self {
        Self {
            entries,
            errors: Vec::new(),
        }
    }
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
    EntrySizeInvalid {
        cause: String,
    },
    EntrySeedersNotFound,
    EntrySeedersInvalid {
        cause: ParseIntError,
    },
    EntryLeechersNotFound,
    EntryLeechersInvalid {
        cause: ParseIntError,
    },
    EntryDateNotFound,
    EntryDateInvalid {
        cause: chrono::format::ParseErrorKind,
    },
    EntryMagnetNotFound,
    UnableToQuery {
        url: String,
        cause: String,
    },
    UnableToRead {
        url: String,
        cause: String,
    },
    UnableToBuildUrl {
        cause: ParseError,
    },
}

impl IndexerError {
    pub fn new(origin: &'static str, reason: IndexerErrorReason) -> Self {
        Self { origin, reason }
    }
}

impl std::fmt::Display for IndexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexerError {{ origin={:?}, reason={:?} }}",
            self.origin, self.reason
        )
    }
}

impl std::error::Error for IndexerError {}

#[derive(Clone, Debug)]
pub struct IndexerEntry {
    pub name: String,
    pub url: String,
    pub date: DateTime<Utc>,
    pub size: ByteSize,
    pub seeders: u32,
    pub leechers: u32,
    pub magnet: String,
    pub origin: &'static str,
}

impl IndexerEntry {
    pub fn date_str(&self) -> String {
        self.date.to_rfc2822()
    }

    pub fn size_str(&self) -> String {
        self.size.as_u64().to_string()
    }
}
