use manteau_indexer_prelude::{IndexerError, IndexerErrorReason};
use url::Url;

pub fn create_magnet(name: &str, info_hash: &str) -> Result<String, IndexerError> {
    let xt = format!("urn:bith:{info_hash}");
    Url::parse_with_params(
        "magnet:",
        [
            ("xt", xt.as_str()),
            ("dn", name.trim()),
            ("tr", "udp://tracker.coppersurfer.tk:6969/announce"),
            ("tr", "udp://tracker.openbittorrent.com:6969/announce"),
            ("tr", "udp://9.rarbg.to:2710/announce"),
            ("tr", "udp://9.rarbg.to:2780/announce"),
            ("tr", "udp://9.rarbg.to:2730/announce"),
            ("tr", "udp://tracker.opentrackr.org:1337"),
            ("tr", "http://p4p.arenabg.com:1337/announce"),
            ("tr", "udp://tracker.torrent.eu.org:451/announce"),
            ("tr", "udp://tracker.tiny-vps.com:6969/announce"),
            ("tr", "udp://open.stealth.si:80/announce"),
        ],
    )
    .map(|url| url.to_string())
    .map_err(|_err| IndexerError::new(super::NAME, IndexerErrorReason::EntryMagnetNotFound))
}
