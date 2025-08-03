use std::{fmt, str::FromStr};

use anyhow::bail;

pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl FromStr for DateTime {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((date, time)) = s.split_once('T') else {
            bail!("missing `T` in datetime");
        };
        Ok(Self {
            date: date.parse()?,
            time: time.parse()?,
        })
    }
}

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

pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc: bool,
}

impl FromStr for Time {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let Some((hour, input)) = input.split_at_checked(2) else {
            bail!("non-ascii time");
        };
        let hour = hour.parse()?;

        let Some((minute, input)) = input.split_at_checked(2) else {
            bail!("non-ascii time");
        };
        let minute = minute.parse()?;

        let Some((second, utc)) = input.split_at_checked(2) else {
            bail!("non-ascii time");
        };
        let second = second.parse()?;

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

// Duration

pub struct Duration {
    pub negative: bool,
    pub kind: DurationKind,
}

impl FromStr for Duration {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (negative, input) = plus_or_minus(input);
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

fn plus_or_minus(input: &str) -> (bool, &str) {
    let mut iter = input.chars();
    match iter.next() {
        Some('+') => (false, iter.as_str()),
        Some('-') => (true, iter.as_str()),
        _ => (false, input),
    }
}

#[cfg(test)]
mod tests {
    use super::Date;

    #[test]
    fn format_date() {
        let date = Date {
            full_year: 500,
            month: 2,
            day: 1,
        };
        assert_eq!(date.to_string(), "0500-02-01");
    }
}
