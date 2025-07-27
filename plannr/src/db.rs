use std::borrow::Cow;

use anyhow::bail;
use oauth2::{EmptyExtraTokenFields, StandardTokenResponse, basic::BasicTokenType};
use sqlx::{SqliteConnection, SqliteExecutor};

use crate::data::{Calendar, Event, EventInterval, EventIntervalRef, RowID};

pub async fn get_calendars(exec: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Calendar>> {
    sqlx::query_as!(Calendar, "SELECT id, name FROM calendars")
        .fetch_all(exec)
        .await
}

pub async fn get_calendar(
    calendar_id: RowID,
    exec: impl SqliteExecutor<'_>,
) -> sqlx::Result<Option<Calendar>> {
    sqlx::query_as!(
        Calendar,
        "SELECT id, name FROM calendars WHERE id = ?",
        calendar_id
    )
    .fetch_optional(exec)
    .await
}

pub async fn find_calendar(name: &str, exec: impl SqliteExecutor<'_>) -> anyhow::Result<Calendar> {
    let like_input = format!("%{}%", escape_like(name));
    tracing::debug!("Input to LIKE statment: `{like_input}`");
    let calendars = sqlx::query_as!(
        Calendar,
        "SELECT id, name FROM calendars WHERE name LIKE ?",
        like_input
    )
    .fetch_all(exec)
    .await?;
    match calendars.len() {
        0 => bail!("no calendars matched `{name}`"),
        1 => Ok(calendars.into_iter().next().unwrap()),
        _ => {
            if let Some(calendar) = calendars
                .into_iter()
                .find(|cal| cal.name.eq_ignore_ascii_case(name))
            {
                Ok(calendar)
            } else {
                bail!("`{name}` is ambiguous as calendar name")
            }
        }
    }
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

pub async fn get_events(
    calendar_id: Option<RowID>,
    exec: impl SqliteExecutor<'_>,
) -> anyhow::Result<Vec<Event>> {
    Ok(if let Some(calendar_id) = calendar_id {
        // TODO if we use a custom type for raw event we could share code between branches
        let raw = sqlx::query!("SELECT id, calendar_id, label, start_time, end_time, date_only FROM events WHERE calendar_id = ?", calendar_id)
            .fetch_all(exec)
            .await?;
        raw.into_iter()
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
            .collect::<Result<Vec<_>, sqlx::Error>>()
    } else {
        let raw = sqlx::query!(
            "SELECT id, calendar_id, label, start_time, end_time, date_only FROM events"
        )
        .fetch_all(exec)
        .await?;
        raw.into_iter()
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
            .collect::<Result<Vec<_>, sqlx::Error>>()
    }?)
}

pub async fn get_events_for_calendar(
    exec: impl SqliteExecutor<'_>,
    calendar_id: RowID,
) -> anyhow::Result<Vec<Event>> {
    let rows =
        sqlx::query!("SELECT id, calendar_id, label, start_time, end_time, date_only FROM events WHERE calendar_id = ?", calendar_id)
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

pub async fn get_events_for_calendars(
    exec: impl SqliteExecutor<'_> + Copy,
    calendar_ids: &[RowID],
) -> anyhow::Result<Vec<Event>> {
    let mut events = vec![];
    for cal_id in calendar_ids {
        events.extend(get_events_for_calendar(exec, *cal_id).await?);
    }
    events.sort_by(|left, right| {
        left.interval
            .cmp(&right.interval)
            .then(left.label.cmp(&right.label))
    });
    Ok(events)
}
pub async fn new_event(
    calendar_id: RowID,
    label: &str,
    interval: EventInterval,
    exec: impl SqliteExecutor<'_>,
) -> anyhow::Result<Event> {
    let (start, end, date_only) = match &*interval {
        EventIntervalRef::Date { start, end } => (
            start.with_hms(0, 0, 0).unwrap().as_utc().unix_timestamp(),
            end.with_hms(0, 0, 0).unwrap().as_utc().unix_timestamp(),
            true,
        ),
        EventIntervalRef::DateTime { start, end } => {
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

pub async fn google_token(
    exec: impl SqliteExecutor<'_>,
) -> anyhow::Result<Option<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>>> {
    let raw = sqlx::query!("SELECT token FROM google_oauth")
        .fetch_optional(exec)
        .await?;
    Ok(raw.map(|v| serde_json::from_str(&v.token)).transpose()?)
}
pub async fn store_google_token(
    user: &str,
    token: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    exec: &mut SqliteConnection,
) -> anyhow::Result<()> {
    let token = serde_json::to_string(&token)?;
    sqlx::query!("DELETE FROM google_oauth")
        .execute(&mut *exec)
        .await?;
    sqlx::query!(
        "INSERT INTO google_oauth (user, token) VALUES (?, ?)",
        user,
        token
    )
    .execute(&mut *exec)
    .await?;
    Ok(())
}

/// Assumes a `ESCAPE '\' as part of the LIKE clause`
// TODO could return a Cow and be slightly more efficient, possibly
fn escape_like(input: &str) -> Cow<'_, str> {
    if !input
        .chars()
        .find(|ch| matches!(ch, '%' | '_' | '\\'))
        .is_some()
    {
        return Cow::Borrowed(input);
    }
    let mut output = String::with_capacity(input.len());
    output.extend(input.char_indices().map(|(idx, ch)| match ch {
        '%' => r"\%",
        '_' => r"\_",
        '\\' => r"\\",
        other => &input[idx..idx + other.len_utf8()],
    }));
    Cow::Owned(output)
}
