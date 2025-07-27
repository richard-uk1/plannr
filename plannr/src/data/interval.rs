use serde::{Deserialize, Serialize};
use std::{cmp, fmt, ops};
use thiserror::Error;
use time::{Date, UtcDateTime, error::ComponentRange};

type Result<T, E = EventIntervalError> = std::result::Result<T, E>;

/// Type representing the start and end time of an event
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventInterval {
    inner: EventIntervalRef,
}

impl EventInterval {
    /// Create date-only interval from start and end dates.
    pub fn new_date(start_date: Date, end_date: Date) -> Result<Self> {
        let inner = EventIntervalRef::Date {
            start: start_date,
            end: end_date,
        };
        Self::new_checked(inner)
    }

    /// Create datetime interval from start and end times.
    pub fn new_datetime(start: UtcDateTime, end: UtcDateTime) -> Result<Self> {
        let inner = EventIntervalRef::DateTime { start, end };
        Self::new_checked(inner)
    }

    /// Convert from DB representation to typed repr.
    ///
    /// Should never fail because only validated data should be inserted into DB
    pub(crate) fn from_db(
        start_time: i64,
        end_time: i64,
        date_only: bool,
    ) -> Result<Self, EventIntervalError> {
        let inner = EventIntervalRef::from_db(start_time, end_time, date_only)?;
        Self::new_checked(inner)
    }

    fn new_checked(inner: EventIntervalRef) -> Result<Self> {
        inner.validate()?;
        Ok(Self { inner })
    }
}

impl ops::Deref for EventInterval {
    type Target = EventIntervalRef;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TryFrom<EventIntervalRef> for EventInterval {
    type Error = EventIntervalError;
    fn try_from(value: EventIntervalRef) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self { inner: value })
    }
}

impl fmt::Display for EventInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

#[derive(Debug, Error)]
pub enum EventIntervalError {
    #[error("{0}")]
    Inner(#[from] ComponentRange),
    #[error("end date {end} is before start date {start}")]
    NegativeDateRange { start: Date, end: Date },
    #[error("end time {end} is before start time {start}")]
    NegativeDateTimeRange {
        start: UtcDateTime,
        end: UtcDateTime,
    },
}

/// Event interval
// Note: only ref access provided outside this module to maintain EventInterval variants
// We enforce that this is only available as a ref, not the type system.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventIntervalRef {
    Date {
        start: Date,
        end: Date,
    },
    DateTime {
        start: UtcDateTime,
        end: UtcDateTime,
    },
}

/// Order is only chronological for timezone UTC, as date-only events
/// are interpreted differently in different timezones
///
/// date is (arbitrarily) before datetime
impl Ord for EventIntervalRef {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let res = self.to_datetime().cmp(&other.to_datetime());
        if res != cmp::Ordering::Equal {
            return res;
        }
        match (self.is_date_only(), other.is_date_only()) {
            (true, true) | (false, false) => cmp::Ordering::Equal,
            (true, false) => cmp::Ordering::Less,
            (false, true) => cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for EventIntervalRef {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl EventIntervalRef {
    pub fn from_db(
        start_time: i64,
        end_time: i64,
        date_only: bool,
    ) -> Result<Self, ComponentRange> {
        if date_only {
            let start = UtcDateTime::from_unix_timestamp(start_time)?.date();
            let end = UtcDateTime::from_unix_timestamp(end_time)?.date();
            Ok(Self::Date { start, end })
        } else {
            let start = UtcDateTime::from_unix_timestamp(start_time)?;
            let end = UtcDateTime::from_unix_timestamp(end_time)?;
            Ok(Self::DateTime { start, end })
        }
    }

    fn to_datetime(&self) -> (UtcDateTime, UtcDateTime) {
        match self {
            EventIntervalRef::Date { start, end } => (
                start.with_hms(0, 0, 0).unwrap().as_utc(),
                end.with_hms(0, 0, 0).unwrap().as_utc(),
            ),
            EventIntervalRef::DateTime { start, end } => (*start, *end),
        }
    }

    pub fn is_date_only(&self) -> bool {
        matches!(self, Self::Date { .. })
    }

    fn validate(&self) -> Result<(), EventIntervalError> {
        match *self {
            EventIntervalRef::Date { start, end } => {
                if end < start {
                    return Err(EventIntervalError::NegativeDateRange { start, end });
                }
            }
            EventIntervalRef::DateTime { start, end } => {
                if end < start {
                    return Err(EventIntervalError::NegativeDateTimeRange { start, end });
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for EventIntervalRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventIntervalRef::Date { start, end } => {
                fmt::Display::fmt(start, f)?;
                f.write_str(" - ")?;
                fmt::Display::fmt(end, f)?;
            }
            EventIntervalRef::DateTime { start, end } => {
                fmt::Display::fmt(start, f)?;
                f.write_str(" - ")?;
                fmt::Display::fmt(end, f)?;
            }
        }
        Ok(())
    }
}
