//! Value types in icalendar
use std::{borrow::Cow, fmt, str::FromStr};

use anyhow::bail;
pub use base64::DecodeError;
use base64::{display::Base64Display, prelude::*};
use thiserror::Error;
pub use uriparse::URIError;

use crate::types::{self, time_hour, time_second};

// BINARY

pub struct Binary {
    // Could use `Cow` to allow user to provide buffer
    // if perf was an issue
    // Could also only base64 decode lazily
    pub data: Vec<u8>,
}

impl FromStr for Binary {
    type Err = DecodeError;
    fn from_str(input: &str) -> Result<Self, DecodeError> {
        Ok(Binary {
            data: BASE64_STANDARD.decode(input)?,
        })
    }
}

impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&Base64Display::new(&self.data, &BASE64_STANDARD), f)
    }
}

// BOOLEAN

pub enum Boolean {
    True,
    False,
}

impl FromStr for Boolean {
    type Err = BooleanError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "TRUE" => Ok(Boolean::True),
            "FALSE" => Ok(Boolean::False),
            other => Err(BooleanError(other.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("expected one of `TRUE`, `FALSE`, found {0}")]
pub struct BooleanError(String);

// CAL-ADDRESS

pub struct CalendarUserAddress<'src>(Uri<'src>);

impl<'src> TryFrom<&'src str> for CalendarUserAddress<'src> {
    type Error = URIError;
    fn try_from(input: &'src str) -> Result<Self, Self::Error> {
        Ok(CalendarUserAddress(Uri::try_from(input)?))
    }
}

impl<'src> fmt::Display for CalendarUserAddress<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// DATE

pub struct Date {
    pub first: types::Date,
    pub rest: Vec<types::Date>,
}

impl FromStr for Date {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for entry in &self.rest {
            write!(f, ",{}", entry)?;
        }
        Ok(())
    }
}

// DATE-TIME

pub struct DateTime {
    pub first: types::DateTime,
    pub rest: Vec<types::DateTime>,
}

impl FromStr for DateTime {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, anyhow::Error> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// DURATION

pub struct Duration {
    pub first: types::Duration,
    pub rest: Vec<types::Duration>,
}

impl FromStr for Duration {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, anyhow::Error> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// FLOAT

pub struct Float {
    pub first: f64,
    pub rest: Vec<f64>,
}

impl FromStr for Float {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, anyhow::Error> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// INTEGER

pub struct Integer {
    pub first: i64,
    pub rest: Vec<i64>,
}

impl FromStr for Integer {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// PERIOD

pub struct Period {
    pub first: types::Period,
    pub rest: Vec<types::Period>,
}

impl FromStr for Period {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// RECUR

// TEXT

#[derive(Debug, PartialEq)]
pub struct Text<'src> {
    first: Cow<'src, str>,
    rest: Vec<Cow<'src, str>>,
}

impl<'src> TryFrom<&'src str> for Text<'src> {
    type Error = anyhow::Error;
    fn try_from(input: &'src str) -> Result<Text<'src>, Self::Error> {
        let mut output = Text {
            first: Cow::Borrowed(""),
            rest: vec![],
        };
        let mut iter = input.char_indices().peekable();
        let mut current_start = 0;
        while let Some((idx, ch)) = iter.next() {
            match ch {
                '\\' => match iter.peek().map(|v| *v) {
                    Some((_, ch2 @ '\\' | ch2 @ ',' | ch2 @ ';')) => {
                        iter.next();
                        output.current().to_mut().push(ch2);
                    }
                    Some((_, 'N' | 'n')) => {
                        iter.next();
                        output.current().to_mut().push('\n');
                    }
                    _ => bail!("unexpected character after escape ('\\')"),
                },
                ',' => {
                    output.start_new();
                    current_start = idx + ch.len_utf8();
                }
                ';' => bail!("semicolon should be escaped in text"),
                _ => output.add_to_current(input, current_start, idx, ch),
            }
        }
        Ok(output)
    }
}

impl<'src> Text<'src> {
    fn start_new(&mut self) {
        self.rest.push(Cow::Borrowed(""));
    }
    fn current(&mut self) -> &mut Cow<'src, str> {
        self.rest.last_mut().unwrap_or(&mut self.first)
    }
    fn add_to_current(&mut self, input: &'src str, current_start: usize, idx: usize, ch: char) {
        match self.current() {
            Cow::Borrowed(slice) => *slice = &input[current_start..idx + ch.len_utf8()],
            Cow::Owned(string) => string.push(ch),
        }
    }
}

// TIME

pub struct Time {
    pub first: types::Time,
    pub rest: Vec<types::Time>,
}

impl FromStr for Time {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// URI

pub struct Uri<'src>(uriparse::URI<'src>);

impl<'src> TryFrom<&'src str> for Uri<'src> {
    type Error = URIError;
    fn try_from(input: &'src str) -> Result<Self, Self::Error> {
        Ok(Uri(input.try_into()?))
    }
}

impl<'src> fmt::Display for Uri<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// UTC-OFFSET

pub struct UtcOffset {
    pub negative: bool,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl FromStr for UtcOffset {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some((sign, input)) = input.split_at_checked(1) else {
            bail!("expected `+` or `-`");
        };
        let negative = match sign {
            "+" => false,
            "-" => true,
            _ => bail!("expected `+` or `-`"),
        };
        let (input, hour) = time_hour(input)?;
        let (input, minute) = time_hour(input)?;
        let (input, second) = if !input.is_empty() {
            ("", 0)
        } else {
            time_second(false, input)?
        };
        if !input.is_empty() {
            bail!("trailing characters");
        }
        Ok(UtcOffset {
            negative,
            hour,
            minute,
            second,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Text;

    #[test]
    fn text() {
        let input = r"First text:\,\;\nsecond line\Nthird line,second item\,,";
        let text = Text::try_from(input).unwrap();
        assert_eq!(
            text,
            Text {
                first: "First text:,;\nsecond line\nthird line".into(),
                rest: vec!["second item,".into(), "".into()]
            }
        )
    }

    #[test]
    fn text_should_fail() {
        assert!(Text::try_from(";").is_err());
        assert!(Text::try_from("\\:").is_err());
    }
}
