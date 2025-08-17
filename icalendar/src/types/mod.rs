use std::{fmt, str::FromStr};

use anyhow::bail;
use thiserror::Error;

pub mod recur;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl FromStr for DateTime {
    type Err = StrToDateTimeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((date, time)) = s.split_once('T') else {
            return Err(StrToDateTimeError::MissingT);
        };
        Ok(Self {
            date: date.parse()?,
            time: time.parse()?,
        })
    }
}

#[derive(Debug, Error)]
pub enum StrToDateTimeError {
    #[error("missing `T` in DateTime")]
    MissingT,
    #[error("untyped error")]
    Other(
        #[from]
        #[source]
        anyhow::Error,
    ),
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Full year
    // only positive years allowed
    // date_fullyear
    pub full_year: u16,
    /// Month (01 - 12)
    // date_month
    pub month: u8,
    /// Day of month: 01 - (28 - 31 depending on month)
    // date_mday
    pub day: u8,
}

impl FromStr for Date {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // all ascii so we can use u8,
        let mut iter = input.splitn(3, '-');

        let Some(date_fullyear) = iter.next() else {
            bail!("missing year");
        };
        let full_year = date_fullyear.parse()?;
        let leap_year = full_year % 4 == 0;

        let Some(month) = iter.next() else {
            bail!("missing month");
        };
        let month = month.parse()?;
        match month {
            1..=12 => (),
            _ => bail!("invalid month"),
        }

        let Some(day) = iter.next() else {
            bail!("missing day");
        };
        let day = day.parse()?;
        let max_day = match month {
            2 => {
                if leap_year {
                    29
                } else {
                    28
                }
            }
            // 30 days hath september april june and november
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if !(1..=max_day).contains(&day) {
            bail!("day invalid for given month/year");
        }

        Ok(Self {
            full_year,
            month,
            day,
        })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.full_year, self.month, self.day)
    }
}

// Time

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc: bool,
}

impl FromStr for Time {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (input, hour) = time_hour(input)?;
        let (input, minute) = time_minute(input)?;
        let (utc, second) = time_second(true, input)?;

        let utc = match utc {
            "Z" => true,
            "" => false,
            other => bail!("trailing characters in time: `{other}`"),
        };
        Ok(Self {
            hour,
            minute,
            second,
            utc,
        })
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}{:02}{:02}{}",
            self.hour,
            self.minute,
            self.second,
            if self.utc { "Z" } else { "" }
        )
    }
}

pub(crate) fn time_hour(input: &str) -> anyhow::Result<(&str, u8)> {
    let Some((hour, rest)) = input.split_at_checked(2) else {
        bail!("expected 2 ascii digits");
    };
    let hour = hour.parse()?;
    if hour >= 24 {
        bail!("hours must be in 0-23");
    }
    Ok((rest, hour))
}

pub(crate) fn time_minute(input: &str) -> anyhow::Result<(&str, u8)> {
    let Some((minute, rest)) = input.split_at_checked(2) else {
        bail!("expected 2 ascii digits");
    };
    let minute = minute.parse()?;
    if minute >= 60 {
        bail!("minutes must be in 0-59");
    }
    Ok((rest, minute))
}

pub(crate) fn time_second(include_leap: bool, input: &str) -> anyhow::Result<(&str, u8)> {
    let Some((minute, rest)) = input.split_at_checked(2) else {
        bail!("expected 2 ascii digits");
    };
    let minute = minute.parse()?;
    if include_leap {
        if minute >= 61 {
            bail!("seconds must be in 0-60 (inc leap second");
        }
    } else {
        if minute >= 60 {
            bail!("seconds must be in 0-59");
        }
    }
    Ok((rest, minute))
}

/// Note - ord is not chronological
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DateOrDateTime {
    Date(Date),
    DateTime(DateTime),
}

impl FromStr for DateOrDateTime {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.parse::<DateTime>() {
            Ok(dt) => return Ok(Self::DateTime(dt)),
            Err(StrToDateTimeError::MissingT) => (/* fallthru */),
            Err(StrToDateTimeError::Other(e)) => bail!(e),
        }
        Ok(Self::Date(input.parse()?))
    }
}

impl fmt::Display for DateOrDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateOrDateTime::Date(date) => fmt::Display::fmt(date, f),
            DateOrDateTime::DateTime(date_time) => fmt::Display::fmt(date_time, f),
        }
    }
}

// Duration

pub struct Duration {
    pub negative: bool,
    pub kind: DurationKind,
}

impl FromStr for Duration {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (negative, input) = plus_or_minus_opt(input);
        if let Some(input) = input.strip_prefix("P") {
            let kind = input.parse()?;
            Ok(Duration { negative, kind })
        } else {
            bail!("expected `P`");
        }
    }
}

pub enum DurationKind {
    Weeks(u32),
    DateTime {
        days: u32,
        hours: u32,
        minutes: u32,
        seconds: u32,
    },
}

impl FromStr for DurationKind {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(input) = input.strip_suffix('W') {
            // must be week form
            return Ok(DurationKind::Weeks(input.parse()?));
        }

        let (days, time) = input.split_once('T').unwrap_or((input, ""));
        let days = if days.is_empty() { 0 } else { days.parse()? };
        let (hours, time) = input.split_once('H').unwrap_or(("", time));
        let hours = if hours.is_empty() { 0 } else { hours.parse()? };
        let (minutes, seconds) = input.split_once('M').unwrap_or(("", time));
        let minutes = if minutes.is_empty() {
            0
        } else {
            minutes.parse()?
        };
        let seconds = if seconds.is_empty() {
            0
        } else {
            let Some(seconds) = seconds.strip_suffix('S') else {
                bail!("seconds missing 'S' suffix");
            };
            seconds.parse()?
        };
        Ok(Self::DateTime {
            days,
            hours,
            minutes,
            seconds,
        })
    }
}

// Period

pub enum Period {
    Explicit {
        start: DateTime,
        // TODO invariant, start must be before end.
        end: DateTime,
    },
    Start {
        start: DateTime,
        // TODO invariant: duration should be positive
        duration: Duration,
    },
}

impl FromStr for Period {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some((start, rest)) = input.split_once('/') else {
            bail!("missing '/' in period");
        };
        let start = start.parse()?;
        Ok(if rest.starts_with('P') {
            let duration = rest.parse()?;
            Period::Start { start, duration }
        } else {
            let end = rest.parse()?;
            Period::Explicit { start, end }
        })
    }
}

// Recur

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recur {
    pub freq: recur::Freq,
    pub end: recur::End,
    pub interval: recur::Interval,
}

impl Recur {
    // Needed because comma is used inside the values, but `,FREQ=` is useable as a separator
    pub fn parse_no_freq(input: &str) -> anyhow::Result<Self> {
        let (freq, mut input) = split_once(';', input);
        let freq = freq.parse()?;

        let mut builder = Self::builder(freq);
        while !input.is_empty() {
            let (entry, next_input) = split_once(';', input);
            input = next_input;
            builder.set_param(recur::Param::parse(entry)?)?;
        }
        Ok(builder.build())
    }

    fn builder(freq: recur::Freq) -> recur::Builder {
        recur::Builder::new(freq)
    }
}

impl FromStr for Recur {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some(input) = input.strip_prefix("FREQ=") else {
            bail!("expected `FREQ=`");
        };
        Self::parse_no_freq(input)
    }
}

// checks for a `+` or `-` at the start, defaults to `+` if absent
fn plus_or_minus_opt(input: &str) -> (bool, &str) {
    let mut iter = input.chars();
    match iter.next() {
        Some('+') => (false, iter.as_str()),
        Some('-') => (true, iter.as_str()),
        _ => (false, input),
    }
}

fn split_once(test_ch: char, input: &str) -> (&str, &str) {
    input.split_once(test_ch).unwrap_or((input, ""))
}

#[cfg(test)]
mod tests {
    use super::{Date, Recur, recur};

    #[test]
    fn format_date() {
        let date = Date {
            full_year: 500,
            month: 2,
            day: 1,
        };
        assert_eq!(date.to_string(), "0500-02-01");
    }

    #[test]
    fn recur() {
        let input = "FREQ=YEARLY";
        let recur = input.parse::<Recur>().unwrap();
        assert_eq!(
            recur,
            Recur {
                freq: recur::Freq::Yearly,
                end: recur::End::Forever,
                interval: recur::Interval::new(1).unwrap(),
            }
        );
    }
}
