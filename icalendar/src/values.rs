//! Value types in icalendar
use core::fmt;

use anyhow::bail;
pub use base64::DecodeError;
use base64::{display::Base64Display, prelude::*};
use thiserror::Error;
pub use uriparse::URIError;

use crate::types;

// BINARY

pub struct Binary {
    // Could use `Cow` to allow user to provide buffer
    // if perf was an issue
    // Could also only base64 decode lazily
    pub data: Vec<u8>,
}

impl Binary {
    pub fn parse(input: &str) -> Result<Self, DecodeError> {
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

impl<'a> TryFrom<&'a str> for Boolean {
    type Error = BooleanError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
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

impl<'src> CalendarUserAddress<'src> {
    pub fn parse(input: &'src str) -> Result<Self, URIError> {
        Ok(CalendarUserAddress(Uri::parse(input)?))
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

impl Date {
    pub fn parse(input: &str) -> Result<Self, anyhow::Error> {
        let mut iter = input.split(',');
        // Unwrap: `split` always produces at least 1 value
        let first = iter.next().unwrap().parse()?;
        let rest = iter
            .map(|value| value.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { first, rest })
    }
}

// DATE-TIME

pub struct DateTime {
    pub first: types::DateTime,
    pub rest: Vec<types::DateTime>,
}

impl DateTime {
    pub fn parse(input: &str) -> Result<Self, anyhow::Error> {
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

// FLOAT

// INTEGER

// RECUR

// TEXT

// TIME

// URI

// UTC-OFFSET

pub struct Uri<'src>(uriparse::URI<'src>);

impl<'src> Uri<'src> {
    pub fn parse(input: &'src str) -> Result<Self, URIError> {
        Ok(Uri(input.try_into()?))
    }
}

impl<'src> fmt::Display for Uri<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
