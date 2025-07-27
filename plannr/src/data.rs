use serde::{Deserialize, Serialize};

mod interval;
pub use interval::{EventInterval, EventIntervalError, EventIntervalRef};

pub type RowID = i64;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, cli_table::Table)]
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
