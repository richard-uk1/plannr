use std::{fmt, str::FromStr};

use anyhow::{anyhow, bail};

use crate::{parser::ParserError, types};

use super::Recur;

// "SECONDLY" / "MINUTELY" / "HOURLY" / "DAILY" / "WEEKLY" / "MONTHLY" / "YEARLY"
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Freq {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl FromStr for Freq {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "SECONDLY" => Self::Secondly,
            "MINUTELY" => Self::Minutely,
            "HOURLY" => Self::Hourly,
            "DAILY" => Self::Daily,
            "WEEKLY" => Self::Weekly,
            "MONTHLY" => Self::Monthly,
            "YEARLY" => Self::Yearly,
            _ => bail!(
                "expected one of `SECONDLY`, `MINUTELY`, `HOURLY`, `DAILY`, `WEEKLY`, `MONTHLY`, `YEARLY`"
            ),
        })
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum End {
    Until(types::DateOrDateTime),
    // TODO use u64?
    Count(u32),
    #[default]
    Forever,
}

impl End {
    fn parse_until(input: &str) -> anyhow::Result<Self> {
        Ok(Self::Until(input.parse()?))
    }

    fn parse_count(input: &str) -> anyhow::Result<Self> {
        Ok(Self::Count(input.parse()?))
    }

    // Private helper to format `End` in a `Recur`
    fn fmt(&self) -> Option<impl fmt::Display> {
        if matches!(self, End::Forever) {
            return None;
        }
        struct Display<'a>(&'a End);

        impl<'a> fmt::Display for Display<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    End::Until(date) => write!(f, "UNTIL={date}"),
                    End::Count(count) => write!(f, "COUNT={count}"),
                    End::Forever => unreachable!(),
                }
            }
        }
        Some(Display(&self))
    }
}

/// Non-zero integer (defaults to 1)
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interval(u32);

impl Default for Interval {
    fn default() -> Self {
        Self(1)
    }
}

impl Interval {
    pub fn new(value: u32) -> anyhow::Result<Self> {
        if value == 0 {
            bail!("interval must be positive integer");
        }
        Ok(Self(value))
    }
    pub fn value(self) -> u32 {
        self.0
    }

    pub fn set_value(&mut self, value: u32) -> anyhow::Result<()> {
        *self = Interval::new(value)?;
        Ok(())
    }
}

pub struct BySecond {
    pub first: u8,
    pub rest: Vec<u8>,
}

impl BySecond {
    pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
        let (input, (first, rest)) = _1or2_digit_int_list("second", 0, 59, input)?;
        Ok((input, Self { first, rest }))
    }
}

pub struct ByMinute {
    pub first: u8,
    pub rest: Vec<u8>,
}

impl ByMinute {
    pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
        let (input, (first, rest)) = _1or2_digit_int_list("minute", 0, 59, input)?;
        Ok((input, Self { first, rest }))
    }
}

pub struct ByHour {
    pub first: u8,
    pub rest: Vec<u8>,
}

impl ByHour {
    pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
        let (input, (first, rest)) = _1or2_digit_int_list("hour", 0, 23, input)?;
        Ok((input, Self { first, rest }))
    }
}

// a comma-separated list of 1 or 2 digit integers
fn _1or2_digit_int_list<'a>(
    ty: &'static str,
    min: u8,
    max: u8,
    input: &'a str,
) -> Result<(&'a str, (u8, Vec<u8>)), ParserError> {
    let val_f = _1or2_digit_int(ty, min, max);
    let (mut input, first) = val_f(input)?;
    let mut rest = vec![];
    while let Some(s) = input.strip_prefix(',') {
        let (s, next) = val_f(s)?;
        input = s;
        rest.push(next);
    }
    Ok((input, (first, rest)))
}

/// 1 or 2 digit positive integer
///
/// min and max are inclusive
fn _1or2_digit_int<'a>(
    ty: &'static str,
    min: u8,
    max: u8,
) -> impl Fn(&'a str) -> Result<(&'a str, u8), ParserError> {
    move |input| {
        let (input, val) = take_while_m_n(1, 2, |ch: char| ch.is_ascii_digit(), input)?;
        let val = val.parse()?;
        if val < min || val > max {
            Err(ParserError::out_of_range(ty, min, max, val))
        } else {
            Ok((input, val))
        }
    }
}

// Helper to parse RECUR
pub(crate) struct Builder {
    freq: Freq,
    end: End,
    interval: Interval,
}

impl Builder {
    pub fn new(freq: Freq) -> Self {
        Self {
            freq,
            end: End::default(),
            interval: Interval::default(),
        }
    }
    pub fn set_param(&mut self, param: Param) -> anyhow::Result<()> {
        match param {
            Param::End(end) => self.set_end(end),
        }
    }

    fn set_end(&mut self, end: End) -> anyhow::Result<()> {
        debug_assert_ne!(end, End::Forever, "should only be parsing count or until");
        if !matches!(self.end, End::Forever) {
            bail!("cannot set UNTIL or COUNT more than once");
        }

        self.end = end;
        Ok(())
    }

    pub(crate) fn build(self) -> Recur {
        Recur {
            freq: self.freq,
            end: self.end,
            interval: self.interval,
        }
    }
}

pub(crate) enum Param {
    End(End),
}

impl Param {
    pub(crate) fn parse(input: &str) -> anyhow::Result<Self> {
        let Some((key, val)) = input.split_once('=') else {
            bail!("expected `=`");
        };
        Ok(match key {
            "COUNT" => Param::End(End::parse_count(val)?),
            "UNTIL" => Param::End(End::parse_until(val)?),
            _ => bail!("unexpected RECUR param"),
        })
    }
}

fn take_while_m_n(
    min: usize,
    max: usize,
    pred: impl Fn(char) -> bool,
    input: &str,
) -> Result<(&str, &str), ParserError> {
    let mut chars = input.char_indices();

    for idx in 0..min {
        if !matches!(chars.next(), Some((_idx, v)) if pred(v)) {
            return Err(ParserError::take_while_m_n(
                min,
                max,
                "expected numeric digit",
            ));
        }
    }
    let mut rest = chars.as_str();
    for _ in min..max {
        if !matches!(chars.next(), Some((_idx, v)) if pred(v)) {
            break;
        };
        rest = chars.as_str();
    }
    let first_len = input.len() - rest.len();
    Ok((rest, &input[..first_len]))
}
