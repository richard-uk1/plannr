use core::fmt;

use serde::{Deserialize, Serialize};
use time::{Date, UtcDateTime, error::ComponentRange};

pub type RowID = i64;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, cli_table::Table)]
pub struct Calendar {
    pub id: RowID,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, cli_table::Table)]
pub struct Event {
    pub id: RowID,
    pub calendar_id: RowID,
    pub label: String,
    pub interval: EventInterval,
}

impl Event {
    pub fn from_db(
        id: RowID,
        calendar_id: RowID,
        label: String,
        start_time: i64,
        end_time: i64,
        date_only: bool,
    ) -> Result<Self, sqlx::Error> {
        let interval = EventInterval::from_db(start_time, end_time, date_only)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        Ok(Event {
            id,
            calendar_id,
            label,
            interval,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventInterval {
    Date {
        start: Date,
        end: Date,
    },
    DateTime {
        start: UtcDateTime,
        end: UtcDateTime,
    },
}

impl EventInterval {
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
}

impl fmt::Display for EventInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventInterval::Date { start, end } => {
                fmt::Display::fmt(start, f)?;
                f.write_str(" - ")?;
                fmt::Display::fmt(end, f)?;
            }
            EventInterval::DateTime { start, end } => {
                fmt::Display::fmt(start, f)?;
                f.write_str(" - ")?;
                fmt::Display::fmt(end, f)?;
            }
        }
        Ok(())
    }
}
