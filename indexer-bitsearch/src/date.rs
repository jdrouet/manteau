use chrono::format::ParseErrorKind;
use chrono::{DateTime, NaiveDate, Utc};

pub fn parse(input: &str) -> Result<DateTime<Utc>, ParseErrorKind> {
    NaiveDate::parse_from_str(input, "%b %e, %Y")
        .map(|date| date.and_hms_opt(9, 0, 0).unwrap())
        .map(|dt| dt.and_local_timezone(Utc).unwrap())
        .map_err(|err| err.kind())
}
