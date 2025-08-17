use std::num::ParseIntError;

use thiserror::Error;

/// Couldn't parse the input as an iCalendar document.
// This error is only built if the parse failed (unrecoverable error)
// so we are less bothered about if some variants are large
#[derive(Debug)]
pub struct ParserError {
    // todo span
    kind: ParserErrorKind,
}

#[derive(Debug, Error)]
pub enum ParserErrorKind {
    #[error("expected {expected}")]
    Tag { expected: &'static str },
    #[error("could not parse integer")]
    ParseInt {
        #[from]
        #[source]
        error: ParseIntError,
    },
    #[error("out of range value {value} for {ty} ({min}..={max})")]
    IntOutOfRange {
        ty: &'static str,
        min: i64,
        max: i64,
        value: i64,
    },
    #[error("expected parser to match {min} to {max:?} times")]
    TakeWhileMN {
        min: usize,
        max: Option<usize>,
        #[source]
        inner: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl ParserError {
    pub(crate) fn tag(expected: &'static str) -> Self {
        Self {
            kind: ParserErrorKind::Tag { expected },
        }
    }

    pub(crate) fn out_of_range(
        ty: &'static str,
        min: impl Into<i64>,
        max: impl Into<i64>,
        value: impl Into<i64>,
    ) -> Self {
        Self {
            kind: ParserErrorKind::IntOutOfRange {
                ty,
                min: min.into(),
                max: max.into(),
                value: value.into(),
            },
        }
    }

    pub(crate) fn take_while_m_n(
        min: usize,
        max: usize,
        error: impl Into<Box<dyn std::error::Error + 'static + Send + Sync>>,
    ) -> Self {
        Self {
            kind: ParserErrorKind::TakeWhileMN {
                min,
                max: (max != usize::MAX).then_some(max),
                inner: error.into(),
            },
        }
    }
}

impl From<ParseIntError> for ParserError {
    fn from(error: ParseIntError) -> Self {
        Self {
            kind: ParserErrorKind::ParseInt { error },
        }
    }
}
