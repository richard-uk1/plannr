use std::env;

use anyhow::Result;
use clap::Parser;
use cli_table::{WithTitle, print_stdout};
use plannr::{
    data::EventInterval,
    db::{get_calendars, get_events, new_calendar, new_event},
};
use sqlx::SqlitePool;
use time::{Date, Month, UtcDateTime, macros::format_description};

#[derive(Debug, clap::Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Parser)]
enum Cmd {
    /// Create some entries in the tables for testing
    InitFixtures,
    /// List all calendars
    ListCalendars,
    /// Create a new calendar
    CreateCalendar { name: String },
    /// List all events
    ListEvents,
    /// Create a new event
    CreateEvent {
        calendar_id: i64,
        label: String,
        start_time: String,
        end_time: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    dotenv::dotenv()?;
    match args.cmd {
        Cmd::InitFixtures => init_fixtures().await,
        Cmd::ListCalendars => list_calendars().await,
        Cmd::CreateCalendar { name } => create_calendar(name).await,
        Cmd::ListEvents => list_events().await,
        Cmd::CreateEvent {
            calendar_id,
            label,
            start_time,
            end_time,
        } => create_event(calendar_id, label, start_time, end_time).await,
    }
}

async fn init_fixtures() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar = new_calendar("test calendar name", &mut *conn).await?;
    let interval = EventInterval::Date {
        start: Date::from_calendar_date(2025, Month::July, 6)?,
        end: Date::from_calendar_date(2025, Month::July, 8)?,
    };
    new_event(calendar.id, "example event 1", interval, &mut *conn).await?;
    Ok(())
}

async fn create_calendar(name: String) -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar = new_calendar(&name, &mut *conn).await?;
    print_stdout(vec![calendar].with_title())?;
    Ok(())
}

async fn list_calendars() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendars = get_calendars(&mut *conn).await?;
    print_stdout(calendars.with_title())?;
    Ok(())
}

async fn list_events() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let events = get_events(&mut *conn).await?;
    print_stdout(events.with_title())?;
    Ok(())
}

async fn create_event(
    calendar_id: i64,
    label: String,
    start_time: String,
    end_time: String,
) -> Result<()> {
    let date_desc = format_description!("[year]-[month]-[day]");
    let datetime_desc = format_description!("[year]-[month]-[day] [hour]:[minute]");
    let interval = if let Ok(start) = Date::parse(&start_time, date_desc) {
        // end must be date
        let end = Date::parse(&end_time, date_desc)?;
        EventInterval::Date { start, end }
    } else {
        // try datetime
        let start = UtcDateTime::parse(&start_time, datetime_desc)?;
        let end = UtcDateTime::parse(&end_time, datetime_desc)?;
        EventInterval::DateTime { start, end }
    };
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar = new_event(calendar_id, &label, interval, &mut *conn).await?;
    print_stdout(vec![calendar].with_title())?;
    Ok(())
}
