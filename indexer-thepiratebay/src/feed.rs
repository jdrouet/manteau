use bytesize::ByteSize;
use chrono::format::ParseErrorKind;
use chrono::{DateTime, Utc};
use manteau_indexer_prelude::{IndexerEntry, IndexerError, IndexerErrorReason, IndexerResult};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Entry {
    id: u64,
    info_hash: String,
    // category: u16,
    name: String,
    leechers: u32,
    seeders: u32,
    added: i64,
    size: u64,
}

impl Entry {
    fn size(&self) -> ByteSize {
        ByteSize::b(self.size)
    }

    fn url(&self, base_url: &str) -> String {
        format!("{base_url}/description.php?id={}", self.id)
    }

    fn date(&self) -> Result<DateTime<Utc>, IndexerError> {
        let date = chrono::NaiveDateTime::from_timestamp_opt(self.added, 0).ok_or_else(|| {
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
            size: self.size(),
            seeders: self.seeders,
            leechers: self.leechers,
            magnet: crate::common::create_magnet(self.name.as_str(), self.info_hash.as_str())?,
            origin: super::NAME,
        })
    }
}

async fn fetch(api_url: &str, category: u16) -> Result<Vec<Entry>, IndexerError> {
    let url = format!("{api_url}/precompiled/data_top100_{category}.json");

    let req = reqwest::get(url.as_str()).await.map_err(|err| {
        IndexerError::new(
            super::NAME,
            IndexerErrorReason::UnableToQuery {
                url: url.clone(),
                cause: err.to_string(),
            },
        )
    })?;
    req.json().await.map_err(|err| {
        IndexerError::new(
            super::NAME,
            IndexerErrorReason::UnableToRead {
                url,
                cause: err.to_string(),
            },
        )
    })
}

pub async fn execute(api_url: &str, base_url: &str, categories: &[u16]) -> IndexerResult {
    let category_responses =
        futures::future::join_all(categories.iter().map(|category| fetch(api_url, *category)))
            .await;

    let mut res = IndexerResult::default();
    for category_list in category_responses {
        match category_list {
            Ok(entries) => entries
                .into_iter()
                .for_each(|entry| match entry.try_into(base_url) {
                    Ok(item) => res.entries.push(item),
                    Err(err) => res.errors.push(err),
                }),
            Err(err) => res.errors.push(err),
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_parse_result() {
        let mut server = mockito::Server::new_async().await;

        let search_page = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/precompiled/data_top100_(\d+).json$".to_string()),
            )
            .expect(2)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("./feed.json"))
            .create_async()
            .await;

        let results = execute(
            server.url().as_str(),
            "http://tpb.org",
            &crate::MUSIC_CATEGORIES,
        )
        .await;
        println!("results: {results:#?}");
        assert_eq!(results.entries.len(), 200);
        assert_eq!(results.errors.len(), 0);
        assert_eq!(
            results.entries[0].name,
            "John.Wick.Chapter.4.2023.HDCAM.c1nem4.x264-SUNSCREEN[TGx]"
        );
        assert_eq!(
            results.entries[0].url,
            "http://tpb.org/description.php?id=67117416"
        );
        assert_eq!(results.entries[0].magnet, "magnet:?xt=urn%3Abith%3A19370E3FD96FB1ADA86ED5892BE5B791A2A32254&dn=John.Wick.Chapter.4.2023.HDCAM.c1nem4.x264-SUNSCREEN%5BTGx%5D&tr=udp%3A%2F%2Ftracker.coppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2710%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2780%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2730%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337&tr=http%3A%2F%2Fp4p.arenabg.com%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce");
        assert_eq!(results.entries[0].seeders, 1068);
        assert_eq!(results.entries[0].leechers, 1074);
        assert_eq!(results.entries[0].size.to_string(), "1044.3 MB");
        assert_eq!(
            results.entries[1].name,
            "John Wick Chapter 4 2023.720p.x264.CAMRip.English"
        );

        search_page.assert_async().await;
    }
}
