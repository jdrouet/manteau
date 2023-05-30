use once_cell::sync::Lazy;
use regex::Regex;
use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

static PARSER: Lazy<Regex> = Lazy::new(|| {
    regex::RegexBuilder::new(r"^([0-9]+(?:\.[0-9]+)?)\s*([k]?)$")
        .case_insensitive(true)
        .build()
        .unwrap()
});

#[derive(Debug, Clone)]
pub enum ParseNumberError {
    InvalidFormat,
    InvalidPrefix(String),
    InvalidBaseNumber(ParseIntError),
    InvalidNumber(ParseFloatError),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Base(u32),
    Kilo(f32),
    Mega(f32),
    Giga(f32),
}

fn from_kilo(value: f32) -> usize {
    (value * 1000.0) as usize
}

impl Number {
    pub fn as_value(&self) -> usize {
        match self {
            Self::Base(inner) => *inner as usize,
            Self::Kilo(inner) => from_kilo(*inner),
            Self::Mega(inner) => from_kilo(*inner) * 1000,
            Self::Giga(inner) => from_kilo(*inner) * 1000 * 1000,
        }
    }
}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(found) = PARSER.captures(input) {
            match found.get(2).map(|m| m.as_str()) {
                None | Some("") => {
                    let value = found
                        .get(1)
                        .ok_or(ParseNumberError::InvalidFormat)?
                        .as_str()
                        .parse::<u32>()
                        .map_err(ParseNumberError::InvalidBaseNumber)?;
                    Ok(Self::Base(value))
                }
                Some(other) => {
                    let value = found
                        .get(1)
                        .ok_or(ParseNumberError::InvalidFormat)?
                        .as_str()
                        .parse::<f32>()
                        .map_err(ParseNumberError::InvalidNumber)?;
                    match other {
                        "k" | "K" => Ok(Self::Kilo(value)),
                        "m" | "M" => Ok(Self::Mega(value)),
                        "g" | "G" => Ok(Self::Giga(value)),
                        found => Err(ParseNumberError::InvalidPrefix(found.to_owned())),
                    }
                }
            }
        } else {
            Err(ParseNumberError::InvalidFormat)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_values() {
        assert_eq!(Number::from_str("0").unwrap(), Number::Base(0));
        assert!(matches!(
            Number::from_str("0.2").unwrap_err(),
            ParseNumberError::InvalidBaseNumber(_)
        ));
        assert_eq!(Number::from_str("42").unwrap(), Number::Base(42));
        assert_eq!(Number::from_str("42K").unwrap(), Number::Kilo(42.0));
        assert_eq!(Number::from_str("4.2K").unwrap(), Number::Kilo(4.2));
        assert_eq!(Number::from_str("4.2 K").unwrap(), Number::Kilo(4.2));
        assert_eq!(Number::from_str("4.2 k").unwrap(), Number::Kilo(4.2));
    }

    #[test]
    fn should_convert_to_value() {
        assert_eq!(Number::Base(1).as_value(), 1);
        assert_eq!(Number::Kilo(1.0).as_value(), 1000);
        assert_eq!(Number::Kilo(1.4).as_value(), 1400);
        assert_eq!(Number::Mega(2.0).as_value(), 2000000);
        assert_eq!(Number::Giga(2.0).as_value(), 2000000000);
    }
}
