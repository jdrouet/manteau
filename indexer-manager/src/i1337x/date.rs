use chrono::format::ParseErrorKind;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use once_cell::sync::Lazy;

// 10pm Mar. 21st
static SAME_YEAR_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^(\d+)([ap]m)\s+([A-Za-z]+)\.\s+(\d+)[a-z]+$").unwrap());
// Jul. 18th '20
static WITH_YEAR_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^([A-Za-z]+)\.\s+(\d+)[A-Za-z]+\s+'(\d+)$").unwrap());

fn parse_same_date(input: &str) -> Result<DateTime<Utc>, ParseErrorKind> {
    NaiveTime::parse_from_str(input, "%l:%M%P")
        .map(|time| {
            NaiveDate::default()
                .and_time(time)
                .and_local_timezone(Utc)
                .unwrap()
        })
        .map_err(|err| err.kind())
}

fn parse_close_date(input: &str, cap: regex::Captures) -> Result<DateTime<Utc>, ParseErrorKind> {
    // 10pm Mar. 21st
    // cap[1]=10 cap[2]=pm cap[3]=Mar cap[4]=21
    let year = Utc::now().year();
    // build "10:00pm 2023 Mar 21"
    let value = format!("{}:00{} {year} {} {}", &cap[1], &cap[2], &cap[3], &cap[4]);
    NaiveDateTime::parse_from_str(&value, "%l:%M%P %Y %b %e")
        .map(|dt| dt.and_local_timezone(Utc).unwrap())
        .map_err(|err| {
            tracing::debug!("unable to parse date {input:?}: {err:?}");
            err.kind()
        })
}

fn parse_far_date(input: &str, cap: regex::Captures) -> Result<DateTime<Utc>, ParseErrorKind> {
    // May. 16th '12 => 9:00am 12 May 16
    let value = format!("9:00am {} {} {}", &cap[3], &cap[1], &cap[2]);
    Utc.datetime_from_str(&value, "%l:%M%P %y %b %e")
        .map_err(|err| {
            tracing::debug!("unable to parse date {input:?}: {err:?}");
            err.kind()
        })
}

pub fn parse(input: &str) -> Result<DateTime<Utc>, ParseErrorKind> {
    if let Some(cap) = WITH_YEAR_REGEX.captures(input) {
        // May. 16th '12
        parse_far_date(input, cap)
    } else if let Some(cap) = SAME_YEAR_REGEX.captures(input) {
        // 10pm Mar. 21st
        parse_close_date(input, cap)
    } else {
        // 10:30am
        parse_same_date(input)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_parse_same_date() {
        super::parse("9:30am").unwrap();
        super::parse("3:23pm").unwrap();
    }

    #[test]
    fn should_parse_close_date() {
        super::parse("9am Mar. 21st").unwrap();
        super::parse("3pm Mar. 21st").unwrap();
    }

    #[test]
    fn should_parse_far_date() {
        super::parse("Mar. 3rd '11").unwrap();
        super::parse("Jul. 18th '20").unwrap();
    }

    // #[test]
    // fn should_parse_date_value() {
    //     parse_date_value("9:44am").unwrap();
    //     parse_date_value("1:44pm").unwrap();
    //     parse_date_value("9pm Mar. 21st").unwrap();
    //     parse_date_value("Feb. 1st '03").unwrap();
    //     parse_date_value("Nov. 2nd '05").unwrap();
    //     parse_date_value("Mar. 3rd '11").unwrap();
    //     parse_date_value("Jul. 18th '20").unwrap();
    // }
}
