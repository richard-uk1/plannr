use std::{fmt, str::FromStr};

use anyhow::bail;

use crate::{
    Result,
    parser::ParserError,
    types::{self, VecOne, opt_sign_is_negative},
};

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
    fn parse_until(input: &str) -> Result<Self, ParserError> {
        Ok(Self::Until(input.parse()?))
    }

    fn parse_count(input: &str) -> Result<Self, ParserError> {
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

    fn parse(input: &str) -> Result<Self, ParserError> {
        Ok(Self(input.parse()?))
    }
}

/// type $t must implement debug
macro_rules! impl_comma_list {
    ($name:ident<$t:ty> = $parser:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub VecOne<$t>);

        impl AsRef<VecOne<$t>> for $name {
            fn as_ref(&self) -> &VecOne<$t> {
                &self.0
            }
        }

        impl AsMut<VecOne<$t>> for $name {
            fn as_mut(&mut self) -> &mut VecOne<$t> {
                &mut self.0
            }
        }

        impl $name {
            pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
                let (input, v) = VecOne::parse_comma_separated(input, $parser)?;
                Ok((input, Self(v)))
            }
        }
    };
}

impl_comma_list!(BySecond<u8> = _1or2_digit_int("second", 0, 59));
impl_comma_list!(ByMinute<u8> = _1or2_digit_int("minute", 0, 59));
impl_comma_list!(ByHour<u8> = _1or2_digit_int("hour", 0, 23));
impl_comma_list!(ByWeekDay<WeekDayNum> = WeekDayNum::parse);
impl_comma_list!(ByMonthDay<i8> = monthdaynum);
impl_comma_list!(ByYearDay<i16> = yeardaynum);
impl_comma_list!(ByWeekNo<i8> = ordwk);
impl_comma_list!(ByMonth<u8> = _1or2_digit_int("month", 1, 12));
impl_comma_list!(BySetPos<i16> = yeardaynum);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekDayNum {
    pub week_num: Option<i8>,
    pub weekday: WeekDay,
}

impl WeekDayNum {
    pub fn parse(mut input: &str) -> Result<(&str, Self), ParserError> {
        let week_num = if matches!(input.chars().next(), Some(v) if v.is_ascii_digit() || v == '-' || v == '+')
        {
            let (i, week_num) = ordwk(input)?;
            input = i;
            // `as`: number must be < 128
            Some(week_num)
        } else {
            None
        };
        let (input, weekday) = WeekDay::parse(input)?;
        Ok((input, Self { week_num, weekday }))
    }
}

fn ordwk(input: &str) -> Result<(&str, i8), ParserError> {
    let (input, negative) = opt_sign_is_negative(input);
    let (input, week_num) = _1or2_digit_int("ordwk", 1, 53)(input)?;
    // 53 < i8::MAX
    let week_num = week_num as i8;
    Ok((input, if negative { -week_num } else { week_num }))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeekDay {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl WeekDay {
    pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
        let Some((weekday, rest)) = input.split_at_checked(2) else {
            return Err(ParserError::expected("2-char weekday code"));
        };

        Ok((
            rest,
            match weekday {
                "SU" => Self::Sunday,
                "MO" => Self::Monday,
                "TU" => Self::Tuesday,
                "WE" => Self::Wednesday,
                "TH" => Self::Thursday,
                "FR" => Self::Friday,
                "SA" => Self::Saturday,
                _ => return Err(ParserError::expected("2-char weekday code")),
            },
        ))
    }
}

fn monthdaynum(input: &str) -> Result<(&str, i8), ParserError> {
    let (input, negative) = opt_sign_is_negative(input);
    let (input, num) = _1or2_digit_int("month day", 1, 31)(input)?;
    // num must be < 128
    let num = num as i8;
    Ok((input, if negative { -num } else { num }))
}

fn yeardaynum(input: &str) -> Result<(&str, i16), ParserError> {
    let (input, negative) = opt_sign_is_negative(input);
    let (input, num) = _1to3_digit_int("year day", 1, 366)(input)?;
    // num must be < i16::MAX
    let num = num as i16;
    Ok((input, if negative { -num } else { num }))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekStart(pub WeekDay);

impl WeekStart {
    pub fn parse(input: &str) -> Result<(&str, Self), ParserError> {
        let (input, weekday) = WeekDay::parse(input)?;
        Ok((input, Self(weekday)))
    }
}

impl Default for WeekStart {
    fn default() -> Self {
        Self(WeekDay::Monday)
    }
}

// Helper to parse RECUR

pub(crate) struct Builder {
    freq: Freq,
    end: End,
    interval: Option<Interval>,
    by_second: Option<BySecond>,
    by_minute: Option<ByMinute>,
    by_hour: Option<ByHour>,
    by_week_day: Option<ByWeekDay>,
    by_month_day: Option<ByMonthDay>,
    by_year_day: Option<ByYearDay>,
    by_week_no: Option<ByWeekNo>,
    by_month: Option<ByMonth>,
    by_set_pos: Option<BySetPos>,
    week_start: Option<WeekStart>,
}

macro_rules! set_val {
    ($fn_name:ident($val:ident: $val_ty:ty), $help_name:literal) => {
        fn $fn_name(&mut self, $val: $val_ty) -> Result {
            if let Some(old_val) = self.$val.replace($val) {
                bail!(
                    concat!(
                        "cannot set ",
                        $help_name,
                        " more than once: was already set to {:?}"
                    ),
                    old_val
                );
            }
            Ok(())
        }
    };
}

impl Builder {
    pub fn new(freq: Freq) -> Self {
        Self {
            freq,
            end: End::default(),
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
            week_start: None,
        }
    }
    pub fn set_param(&mut self, param: Param) -> anyhow::Result<()> {
        match param {
            Param::End(end) => self.set_end(end),
            Param::Interval(interval) => self.set_interval(interval),
            Param::BySecond(by_second) => self.set_by_second(by_second),
            Param::ByMinute(by_minute) => self.set_by_minute(by_minute),
            Param::ByHour(by_hour) => self.set_by_hour(by_hour),
            Param::ByWeekDay(by_week_day) => self.set_by_week_day(by_week_day),
            Param::ByMonthDay(by_month_day) => self.set_by_month_day(by_month_day),
            Param::ByYearDay(by_year_day) => self.set_by_year_day(by_year_day),
            Param::ByWeekNo(by_week_no) => self.set_by_week_no(by_week_no),
            Param::ByMonth(by_month) => self.set_by_month(by_month),
            Param::BySetPos(by_set_pos) => self.set_by_set_pos(by_set_pos),
            Param::WeekStart(week_start) => self.set_week_start(week_start),
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

    set_val!(set_interval(interval: Interval), "INTERVAL");
    set_val!(set_by_second(by_second: BySecond), "BYSECOND");
    set_val!(set_by_minute(by_minute: ByMinute), "BYMINUTE");
    set_val!(set_by_hour(by_hour: ByHour), "BYHOUR");
    set_val!(set_by_week_day(by_week_day: ByWeekDay), "BYWEEKDAY");
    set_val!(set_by_month_day(by_month_day: ByMonthDay), "BYMONTHDAY");
    set_val!(set_by_year_day(by_year_day: ByYearDay), "BYYEARDAY");
    set_val!(set_by_week_no(by_week_no: ByWeekNo), "BYWEEKNO");
    set_val!(set_by_month(by_month: ByMonth), "BYMONTH");
    set_val!(set_by_set_pos(by_set_pos: BySetPos), "BYSETPOS");
    set_val!(set_week_start(week_start: WeekStart), "WKST");

    pub(crate) fn build(self) -> Recur {
        Recur {
            freq: self.freq,
            end: self.end,
            interval: self.interval,
            by_second: self.by_second,
            by_minute: self.by_minute,
            by_hour: self.by_hour,
            by_week_day: self.by_week_day,
            by_month_day: self.by_month_day,
            by_year_day: self.by_year_day,
            by_week_no: self.by_week_no,
            by_month: self.by_month,
            by_set_pos: self.by_set_pos,
            week_start: self.week_start,
        }
    }
}

pub(crate) enum Param {
    End(End),
    Interval(Interval),
    BySecond(BySecond),
    ByMinute(ByMinute),
    ByHour(ByHour),
    ByWeekDay(ByWeekDay),
    ByMonthDay(ByMonthDay),
    ByYearDay(ByYearDay),
    ByWeekNo(ByWeekNo),
    ByMonth(ByMonth),
    BySetPos(BySetPos),
    WeekStart(WeekStart),
}

impl Param {
    pub(crate) fn parse(input: &str) -> Result<Self, ParserError> {
        let Some((key, val)) = input.split_once('=') else {
            return Err(ParserError::tag("="));
        };
        Ok(match key {
            "UNTIL" => Param::End(End::parse_until(val)?),
            "COUNT" => Param::End(End::parse_count(val)?),
            "INTERVAL" => Param::Interval(Interval::parse(val)?),
            "BYSECOND" => Param::BySecond(BySecond::parse(val)?.1),
            "BYMINUTE" => Param::ByMinute(ByMinute::parse(val)?.1),
            "BYHOUR" => Param::ByHour(ByHour::parse(val)?.1),
            "BYDAY" => Param::ByWeekDay(ByWeekDay::parse(val)?.1),
            "BYMONTHDAY" => Param::ByMonthDay(ByMonthDay::parse(val)?.1),
            "BYYEARDAY" => Param::ByYearDay(ByYearDay::parse(val)?.1),
            "BYWEEKNO" => Param::ByWeekNo(ByWeekNo::parse(val)?.1),
            "BYMONTH" => Param::ByMonth(ByMonth::parse(val)?.1),
            "BYSETPOS" => Param::BySetPos(BySetPos::parse(val)?.1),
            "WKST" => Param::WeekStart(WeekStart::parse(val)?.1),
            _ => return Err(ParserError::expected("RECUR param")),
        })
    }
}

// helpers

fn take_while_m_n(
    min: usize,
    max: usize,
    pred: impl Fn(char) -> bool,
    input: &str,
) -> Result<(&str, &str), ParserError> {
    let mut chars = input.char_indices();

    for _ in 0..min {
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

/// 1 or 2 digit positive integer
///
/// min and max are inclusive
fn _1or2_digit_int(
    ty: &'static str,
    min: u8,
    max: u8,
) -> impl for<'a> Fn(&'a str) -> Result<(&'a str, u8), ParserError> {
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

/// 1, 2, or 3 digit positive integer
///
/// min and max are inclusive
fn _1to3_digit_int<'a>(
    ty: &'static str,
    min: u16,
    max: u16,
) -> impl Fn(&'a str) -> Result<(&'a str, u16), ParserError> {
    move |input| {
        let (input, val) = take_while_m_n(1, 3, |ch: char| ch.is_ascii_digit(), input)?;
        let val = val.parse()?;
        if val < min || val > max {
            Err(ParserError::out_of_range(ty, min, max, val))
        } else {
            Ok((input, val))
        }
    }
}
