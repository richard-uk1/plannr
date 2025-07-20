use sqlx::SqliteExecutor;

use crate::data::{Calendar, Event, EventInterval, RowID};

pub async fn get_calendars(exec: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Calendar>> {
    sqlx::query_as!(Calendar, "SELECT id, name FROM calendars")
        .fetch_all(exec)
        .await
}

pub async fn new_calendar(name: &str, exec: impl SqliteExecutor<'_>) -> sqlx::Result<Calendar> {
    sqlx::query_as!(
        Calendar,
        "INSERT INTO calendars (name) VALUES (?1) RETURNING id, name",
        name
    )
    .fetch_one(exec)
    .await
}

pub async fn get_events(exec: impl SqliteExecutor<'_>) -> anyhow::Result<Vec<Event>> {
    let rows =
        sqlx::query!("SELECT id, calendar_id, label, start_time, end_time, date_only FROM events")
            .fetch_all(exec)
            .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            Event::from_db(
                row.id,
                row.calendar_id,
                row.label,
                row.start_time,
                row.end_time,
                row.date_only,
            )
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?)
}

pub async fn new_event(
    calendar_id: RowID,
    label: &str,
    interval: EventInterval,
    exec: impl SqliteExecutor<'_>,
) -> anyhow::Result<Event> {
    let (start, end, date_only) = match interval {
        EventInterval::Date { start, end } => (
            start.with_hms(0, 0, 0).unwrap().as_utc().unix_timestamp(),
            end.with_hms(0, 0, 0).unwrap().as_utc().unix_timestamp(),
            true,
        ),
        EventInterval::DateTime { start, end } => {
            (start.unix_timestamp(), end.unix_timestamp(), false)
        }
    };
    let row = sqlx::query!(
        "INSERT INTO events (calendar_id, label, start_time, end_time, date_only) \
        VALUES (?, ?, ?, ?, ?) \
        RETURNING id, calendar_id, label, start_time, end_time, date_only",
        calendar_id,
        label,
        start,
        end,
        date_only
    )
    .fetch_one(exec)
    .await?;
    Ok(Event::from_db(
        row.id,
        row.calendar_id,
        row.label,
        row.start_time,
        row.end_time,
        row.date_only,
    )?)
}
