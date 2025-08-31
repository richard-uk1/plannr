//! Types that are contained in either values or params
use std::{fmt, str::FromStr};

use anyhow::bail;

use crate::{
    Result,
    parser::{
        ParserError,
        helpers::{_1or2_digit_int, _1to4_digit_int, parse_u32, tag},
    },
};

pub mod recur;

mod vec_one;
pub use vec_one::VecOne;

mod name;
pub use name::{Name, XName};

mod location;
pub use location::GeoLocation;

mod priority;
pub use priority::Priority;

mod data;
pub use data::Data;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl DateTime {
    pub fn parse(input: &str) -> Result<(&str, Self)> {
        let (input, date) = Date::parse(input)?;
        let (input, _) = tag("T")(input)?;
        let (input, time) = Time::parse(input)?;
        Ok((input, DateTime { date, time }))
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {:?}", self.date, self.time)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl Date {
    pub(crate) fn parse(input: &str) -> Result<(&str, Self)> {
        // all ascii so we can use u8,

        let (input, full_year) = _1to4_digit_int("year", u16::MIN, u16::MAX)(input)?;
        let leap_year = full_year % 4 == 0;

        let (input, month) = _1or2_digit_int("month", 1, 12)(input)?;

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
        let (input, day) = _1or2_digit_int("day", 1, max_day)(input)?;

        Ok((
            input,
            Self {
                full_year,
                month,
                day,
            },
        ))
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}{:02}{:02}", self.full_year, self.month, self.day)
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.full_year, self.month, self.day)
    }
}

// Time

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc: bool,
}

impl Time {
    pub fn parse(input: &str) -> Result<(&str, Self)> {
        let (input, hour) = time_hour(input)?;
        let (input, minute) = time_minute(input)?;
        let (input, second) = time_second(true, input)?;

        let mut iter = input.chars();
        let (input, utc) = match iter.next() {
            Some('Z') => (iter.as_str(), true),
            _ => (input, false),
        };
        Ok((
            input,
            Self {
                hour,
                minute,
                second,
                utc,
            },
        ))
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

impl fmt::Debug for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}{}",
            self.hour,
            self.minute,
            self.second,
            if self.utc { " Z" } else { "" }
        )
    }
}

pub(crate) fn time_hour(input: &str) -> Result<(&str, u8), ParserError> {
    let Some((hour, rest)) = input.split_at_checked(2) else {
        return Err(ParserError::expected("2 ascii digits"));
    };
    let hour = hour.parse()?;
    if hour >= 24 {
        return Err(ParserError::out_of_range("hour", 0, 23, hour));
    }
    Ok((rest, hour))
}

pub(crate) fn time_minute(input: &str) -> Result<(&str, u8), ParserError> {
    let Some((minute, rest)) = input.split_at_checked(2) else {
        return Err(ParserError::expected("2 ascii digits"));
    };
    let minute = minute.parse()?;
    if minute >= 60 {
        return Err(ParserError::out_of_range("minute", 0, 60, minute));
    }
    Ok((rest, minute))
}

pub(crate) fn time_second(include_leap: bool, input: &str) -> Result<(&str, u8), ParserError> {
    let Some((seconds, rest)) = input.split_at_checked(2) else {
        return Err(ParserError::expected("2 ascii digits"));
    };
    let seconds = seconds.parse()?;
    if include_leap {
        if seconds >= 61 {
            return Err(ParserError::out_of_range("seconds", 0, 60, seconds));
        }
    } else {
        if seconds >= 60 {
            return Err(ParserError::out_of_range("seconds", 0, 59, seconds));
        }
    }
    Ok((rest, seconds))
}

/// Note - ord is not chronological
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DateOrDateTime {
    Date(Date),
    DateTime(DateTime),
}

impl DateOrDateTime {
    pub(crate) fn parse(input: &str) -> Result<(&str, Self)> {
        let (input, date) = Date::parse(input)?;
        if matches!(input.chars().next(), Some('T')) {
            let (input, _) = tag("T")(input)?;
            let (input, time) = Time::parse(input)?;
            Ok((input, DateOrDateTime::DateTime(DateTime { date, time })))
        } else {
            Ok((input, DateOrDateTime::Date(date)))
        }
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

impl fmt::Debug for DateOrDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateOrDateTime::Date(date) => fmt::Debug::fmt(date, f),
            DateOrDateTime::DateTime(date_time) => fmt::Debug::fmt(date_time, f),
        }
    }
}

// Duration

#[derive(Debug)]
pub struct Duration {
    pub negative: bool,
    pub kind: DurationKind,
}

impl Duration {
    pub(crate) fn parse(input: &str) -> Result<(&str, Self)> {
        let (input, negative) = opt_sign_is_negative(input);
        let (input, _) = tag("P")(input)?;
        let (input, kind) = DurationKind::parse(input)?;
        Ok((input, Duration { negative, kind }))
    }
}

#[derive(Debug)]
pub enum DurationKind {
    Weeks(u32),
    DateTime {
        days: u32,
        hours: u32,
        minutes: u32,
        seconds: u32,
    },
}

impl DurationKind {
    fn parse(mut input: &str) -> Result<(&str, Self)> {
        fn parse_mins_secs<'a>(
            input: &'a str,
            minutes: &mut u32,
            seconds: &mut u32,
        ) -> Result<&'a str> {
            let Some((i, num)) = parse_u32(input)? else {
                return Ok(input);
            };
            let Ok((i, _)) = tag("M")(i) else {
                return Ok(input);
            };
            *minutes = num;
            parse_secs(i, seconds)
        }

        fn parse_secs<'a>(input: &'a str, seconds: &mut u32) -> Result<&'a str> {
            let Some((i, num)) = parse_u32(input)? else {
                return Ok(input);
            };
            let Ok((i, _)) = tag("S")(i) else {
                return Ok(input);
            };
            *seconds = num;
            Ok(i)
        }

        let mut days = 0;
        let mut hours = 0;
        let mut minutes = 0;
        let mut seconds = 0;

        if !matches!(input.chars().next(), Some('T')) {
            // there must be days
            let Some((i, num)) = parse_u32(input)? else {
                bail!("expected integer or `T`");
            };
            let mut iter = i.chars();
            match iter.next() {
                Some('W') => {
                    return Ok((iter.as_str(), DurationKind::Weeks(num)));
                }
                Some('D') => {
                    days = num;
                    input = iter.as_str();
                }
                _ => bail!("expected `W` or `D`"),
            }
        }
        let (input, _) = tag("T")(input)?;
        let Some((mut input, num)) = parse_u32(input)? else {
            bail!("expected integer");
        };
        let mut iter = input.chars();
        match iter.next() {
            Some('H') => {
                hours = num;
                input = parse_mins_secs(iter.as_str(), &mut minutes, &mut seconds)?;
            }
            Some('M') => {
                minutes = num;
                input = parse_secs(iter.as_str(), &mut seconds)?;
            }
            Some('S') => {
                seconds = num;
                input = iter.as_str();
            }
            _ => bail!("expected one of `H`, `M`, `S`"),
        }

        Ok((
            input,
            DurationKind::DateTime {
                days,
                hours,
                minutes,
                seconds,
            },
        ))
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

impl Period {
    fn parse(input: &str) -> Result<(&str, Self)> {
        let (input, start) = DateTime::parse(input)?;
        let (input, _) = tag("/")(input)?;
        Ok(if input.starts_with('P') {
            let (input, duration) = Duration::parse(input)?;
            (input, Period::Start { start, duration })
        } else {
            let (input, end) = DateTime::parse(input)?;
            (input, Period::Explicit { start, end })
        })
    }
}

// Recur

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recur {
    pub freq: recur::Freq,
    pub end: recur::End,
    pub interval: Option<recur::Interval>,
    pub by_second: Option<recur::BySecond>,
    pub by_minute: Option<recur::ByMinute>,
    pub by_hour: Option<recur::ByHour>,
    pub by_week_day: Option<recur::ByWeekDay>,
    pub by_month_day: Option<recur::ByMonthDay>,
    pub by_year_day: Option<recur::ByYearDay>,
    pub by_week_no: Option<recur::ByWeekNo>,
    pub by_month: Option<recur::ByMonth>,
    pub by_set_pos: Option<recur::BySetPos>,
    pub week_start: Option<recur::WeekStart>,
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
fn opt_sign_is_negative(input: &str) -> (&str, bool) {
    let mut iter = input.chars();
    match iter.next() {
        Some('+') => (iter.as_str(), false),
        Some('-') => (iter.as_str(), true),
        _ => (input, false),
    }
}

fn split_once(test_ch: char, input: &str) -> (&str, &str) {
    input.split_once(test_ch).unwrap_or((input, ""))
}

#[cfg(test)]
mod tests {

    use super::{Date, DateTime, Recur, recur};

    #[test]
    fn format_date() {
        let date = Date {
            full_year: 500,
            month: 2,
            day: 1,
        };
        assert_eq!(date.to_string(), "05000201");
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
                interval: None,
                by_second: None,
                by_minute: None,
                by_hour: None,
                by_week_day: None,
                by_month_day: None,
                by_year_day: None,
                by_week_no: None,
                by_month: None,
                by_set_pos: None,
                week_start: None
            }
        );
    }

    #[test]
    fn date_time() {
        let input = "20111217T152336Z";
        let (input, parsed) = DateTime::parse(input).unwrap();
        assert_eq!(
            parsed,
            super::DateTime {
                date: super::Date {
                    full_year: 2011,
                    month: 12,
                    day: 17,
                },
                time: super::Time {
                    hour: 15,
                    minute: 23,
                    second: 36,
                    utc: true
                }
            }
        );
        assert_eq!(input, "");
    }
}
