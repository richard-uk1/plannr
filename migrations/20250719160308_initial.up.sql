CREATE TABLE calendars (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    calendar_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    -- Use i64 to ensure no database shenanigans (unix timestamp in UTC)
    start_time INTEGER NOT NULL,
    end_time INTEGER NOT NULL,
    -- When true time is truncated from start/end_time
    date_only BOOLEAN NOT NULL
)
