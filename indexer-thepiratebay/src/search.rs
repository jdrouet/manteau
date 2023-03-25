use bytesize::ByteSize;
use chrono::format::ParseErrorKind;
use chrono::{DateTime, Utc};
use manteau_indexer_prelude::{IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult};
use url::Url;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Entry {
    id: String,
    name: String,
    info_hash: String,
    leechers: String,
    seeders: String,
    added: String,
    // num_files: String,
    size: String,
    // category: String,
    // imdb: Option<String>,
}

impl Entry {
    fn size(&self) -> Result<ByteSize, IndexerError> {
        let size = self.size.parse::<u64>().map_err(|err| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntrySizeInvalid {
                    cause: err.to_string(),
                },
            )
        })?;
        Ok(ByteSize::b(size))
    }

    fn seeders(&self) -> Result<u32, IndexerError> {
        self.seeders.parse::<u32>().map_err(|cause| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntrySeedersInvalid { cause },
            )
        })
    }

    fn leechers(&self) -> Result<u32, IndexerError> {
        self.leechers.parse::<u32>().map_err(|cause| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntryLeechersInvalid { cause },
            )
        })
    }

    fn url(&self, base_url: &str) -> String {
        format!("{base_url}/description.php?id={}", self.id)
    }

    fn date(&self) -> Result<DateTime<Utc>, IndexerError> {
        let timestamp = self.added.parse::<i64>().map_err(|_err| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntryDateInvalid {
                    cause: ParseErrorKind::BadFormat,
                },
            )
        })?;
        let date = chrono::NaiveDateTime::from_timestamp_opt(timestamp, 0).ok_or_else(|| {
            IndexerError::new(
                super::NAME,
                IndexerErrorReason::EntryDateInvalid {
                    cause: ParseErrorKind::Impossible,
                },
            )
        })?;
        Ok(DateTime::from_utc(date, Utc))
    }

    pub(crate) fn try_into(self, base_url: &str) -> Result<IndexerEntry, IndexerError> {
        Ok(IndexerEntry {
            name: self.name.trim().to_string(),
            url: self.url(base_url),
            date: self.date()?,
            size: self.size()?,
            seeders: self.seeders()?,
            leechers: self.leechers()?,
            magnet: crate::common::create_magnet(self.name.as_str(), self.info_hash.as_str())?,
            origin: super::NAME,
        })
    }
}

async fn fetch(base_url: &str, query: &str, category: u16) -> Result<Vec<Entry>, IndexerError> {
    let url = Url::parse_with_params(
        format!("{base_url}/q.php").as_str(),
        &[("q", query), ("cat", category.to_string().as_str())],
    )
    .map_err(|cause| {
        IndexerError::new(super::NAME, IndexerErrorReason::UnableToBuildUrl { cause })
    })?;
    let url_str = url.to_string();

    let req = reqwest::get(url).await.map_err(|err| {
        IndexerError::new(
            super::NAME,
            IndexerErrorReason::UnableToQuery {
                url: url_str.clone(),
                cause: err.to_string(),
            },
        )
    })?;
    req.json().await.map_err(|err| {
        IndexerError::new(
            super::NAME,
            IndexerErrorReason::UnableToRead {
                url: url_str,
                cause: err.to_string(),
            },
        )
    })
}

pub async fn execute(base_url: &str, query: &str, category: u16) -> IndexerResult {
    let entries = match fetch(base_url, query, category).await {
        Ok(value) => value,
        Err(error) => return IndexerResult::from(error),
    };

    let mut res = IndexerResult::default();
    entries
        .into_iter()
        .for_each(|entry| match entry.try_into(base_url) {
            Ok(item) => res.entries.push(item),
            Err(err) => res.errors.push(err),
        });
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_parse_result() {
        let mut server = mockito::Server::new_async().await;

        let search_page = server
            .mock("GET", "/q.php?q=how+i+met+your+mother&cat=0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("./search.json"))
            .create_async()
            .await;

        let results = execute(server.url().as_str(), "how i met your mother", 0).await;
        println!("results: {results:#?}");
        assert_eq!(results.entries.len(), 100);
        assert_eq!(results.errors.len(), 0);
        assert_eq!(
            results.entries[0].name,
            "How I Met Your Mother Season 1 S01 (1080p Web x265 HEVC AAC 5.1"
        );
        assert_eq!(results.entries[0].seeders, 97);
        assert_eq!(results.entries[0].leechers, 89);
        assert_eq!(results.entries[0].size.to_string(), "3.7 GB");
        assert_eq!(
            results.entries[1].name,
            "How I Met Your Mother Season 2 S02 (1080p Web x265 HEVC AAC 5.1"
        );

        search_page.assert_async().await;
    }
}
