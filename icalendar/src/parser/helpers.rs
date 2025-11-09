//! Generic helpers for parsing

use std::borrow::Cow;

use anyhow::{anyhow, bail};
use thiserror::Error;

use crate::{
    Result,
    parser::{ParserError, VALUE_PARAM, line::Line},
    types::{Date, DateOrDateTime, DateTime, VecOne},
};

pub fn param_value<'src>(input: Cow<'src, str>) -> Result<Cow<'src, str>> {
    if input.starts_with('"') {
        quoted_string(input)
    } else {
        check_param_text(&input)?;
        Ok(input)
    }
}

pub fn check_param_text<'src>(input: &'src str) -> Result<()> {
    for ch in input.chars() {
        safe_char(ch)?;
    }
    Ok(())
}

/// Returns `input` without the start and end quotes
pub fn quoted_string(input: Cow<'_, str>) -> anyhow::Result<Cow<'_, str>> {
    let mut iter = input.chars();
    if !matches!(iter.next(), Some('"')) {
        bail!("quoted string must start with `\"`");
    }
    if !matches!(iter.next_back(), Some('"')) {
        bail!("quoted string must end with `\"`");
    }
    for ch in iter {
        qsafe_char(ch)?;
    }
    // pop front and back quotes
    Ok(match input {
        Cow::Borrowed(input) => Cow::Borrowed(input.trim_matches('"')),
        Cow::Owned(mut input) => {
            input.pop();
            // big copy, but doesn't allocate
            input.remove(0);
            Cow::Owned(input)
        }
    })
}

pub fn safe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        ch @ '"' | ch @ ';' | ch @ ':' | ch @ ',' => bail!("`{ch}` not allowed"),
        _ => Ok(()),
    }
}

pub fn qsafe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        '"' => bail!("`\"` is not allowed"),
        _ => Ok(()),
    }
}

/// Like the std library equivalent, except: owned -> owned (involves allocation)
///
/// returns original string if split was not possible
pub fn try_split_once<'a>(
    input: Cow<'a, str>,
    delim: char,
) -> Result<(Cow<'a, str>, Cow<'a, str>), Cow<'a, str>> {
    match input {
        Cow::Borrowed(s) => {
            let Some((before, after)) = s.split_once(delim) else {
                return Err(Cow::Borrowed(s));
            };
            Ok((Cow::Borrowed(before), Cow::Borrowed(after)))
        }
        Cow::Owned(mut s) => {
            let Some(split_idx) = s.find(delim) else {
                return Err(Cow::Owned(s));
            };
            let after = s.split_off(split_idx + delim.len_utf8());
            // pop delimiter
            s.pop();
            Ok((Cow::Owned(s), Cow::Owned(after)))
        }
    }
}

/// Like the std library equivalent, except:
///  1. owned -> owned (involves allocation)
///  2. if no delim then return all in first part
pub fn split_once<'a>(input: Cow<'a, str>, delim: char) -> (Cow<'a, str>, Cow<'a, str>) {
    match input {
        Cow::Borrowed(s) => match s.split_once(delim) {
            Some((before, after)) => (Cow::Borrowed(before), Cow::Borrowed(after)),
            None => (input, Cow::Borrowed("")),
        },
        Cow::Owned(mut s) => {
            let Some(split_idx) = s.find(delim) else {
                return (Cow::Owned(s), Cow::Borrowed(""));
            };
            let after = s.split_off(split_idx + delim.len_utf8());
            // pop delimiter
            s.pop();
            (Cow::Owned(s), Cow::Owned(after))
        }
    }
}

/// Split on the given character, or return everything in `.0` if it isn't
/// found.
///
/// This function will only split outside quoted strings (i.e. when the
/// number of seen quotes is odd).
pub fn split_once_outside_quotes<'a>(
    input: Cow<'a, str>,
    test_ch: char,
) -> (Cow<'a, str>, Cow<'a, str>) {
    match input {
        Cow::Borrowed(input) => split_once_outside_quotes_borrowed(input, test_ch),
        Cow::Owned(input) => split_once_outside_quotes_owned(input, test_ch),
    }
}

fn split_once_outside_quotes_borrowed<'a>(
    input: &'a str,
    test_ch: char,
) -> (Cow<'a, str>, Cow<'a, str>) {
    let mut chars = input.chars();
    let mut inside_quote = false;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            inside_quote = !inside_quote;
        } else if ch == test_ch && !inside_quote {
            let rest = chars.as_str();
            let first = &input[..input.len() - rest.len() - test_ch.len_utf8()];
            return (Cow::Borrowed(first), Cow::Borrowed(rest));
        }
    }
    (Cow::Borrowed(input), Cow::Borrowed(""))
}

fn split_once_outside_quotes_owned(
    mut input: String,
    test_ch: char,
) -> (Cow<'static, str>, Cow<'static, str>) {
    let mut chars = input.chars();
    let mut inside_quote = false;
    let mut first_end_idx = None;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            inside_quote = !inside_quote;
        } else if ch == test_ch && !inside_quote {
            let rest = chars.as_str();
            first_end_idx = Some(input.len() - rest.len());
            break;
        }
    }
    let Some(first_end_idx) = first_end_idx else {
        return (Cow::Owned(input), Cow::Borrowed(""));
    };
    let after = input.split_off(first_end_idx);
    // remove `test_ch`
    input.pop();
    (Cow::Owned(input), Cow::Owned(after))
}

pub fn pop_front_bytes(input: &mut String, chars: usize) {
    let mut finished = false;
    let mut count = 0;
    input.retain(|_ch| {
        if finished {
            return true;
        }
        if count >= chars {
            finished = true;
            return true;
        }
        count += 1;
        false
    });
}

/// parse `iana-token` (anything matching BNF not just registered tokens)
pub fn check_iana_token(input: &str) -> Result {
    if input.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
        Ok(())
    } else {
        // we use the term 'name' because it is easier to understand
        Err(anyhow!("{input} is not a valid name"))
    }
}

/// 1 or 2 digit positive integer
///
/// min and max are inclusive
pub fn _1or2_digit_int(
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
pub fn _1to3_digit_int<'a>(
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

/// 1-4 digit positive integer
///
/// min and max are inclusive
pub fn _1to4_digit_int<'a>(
    ty: &'static str,
    min: u16,
    max: u16,
) -> impl Fn(&'a str) -> Result<(&'a str, u16), ParserError> {
    move |input| {
        let (input, val) = take_while_m_n(1, 4, |ch: char| ch.is_ascii_digit(), input)?;
        let val = val.parse()?;
        if val < min || val > max {
            Err(ParserError::out_of_range(ty, min, max, val))
        } else {
            Ok((input, val))
        }
    }
}

pub fn take_while_m_n(
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

pub fn strip_prefix<'a>(
    input: Cow<'a, str>,
    prefix: &'static str,
) -> Result<Cow<'a, str>, Cow<'a, str>> {
    if input.starts_with(prefix) {
        Ok(match input {
            Cow::Owned(mut s) => {
                s.replace_range(0..prefix.len(), "");
                Cow::Owned(s)
            }
            Cow::Borrowed(s) => Cow::Borrowed(s.strip_prefix(prefix).unwrap()),
        })
    } else {
        Err(input)
    }
}

pub fn opt_vec_one_to_vec<T>(input: Option<VecOne<T>>) -> Vec<T> {
    let mut v = vec![];
    match input {
        Some(vec_one) => {
            v.push(vec_one.first);
            v.extend(vec_one.rest);
        }
        None => (),
    }
    v
}

/// Expect a datetime value, unless there is the parameter VALUE=DATE, in which
/// case date instead
pub fn parse_date_or_datetime(input: &mut Line<'_>) -> Result<DateOrDateTime> {
    let is_datetime = input
        .params
        .take(&VALUE_PARAM)
        .map(|v| -> Result<_> {
            let v = v.get_single()?;
            Ok(match &*v {
                "DATE-TIME" => true,
                "DATE" => false,
                other => bail!("unexpected recurrance id VALUE param {other}"),
            })
        })
        .transpose()?
        .unwrap_or(true);

    Ok(if is_datetime {
        let (_, datetime) = DateTime::parse(&*input.value)?;
        DateOrDateTime::DateTime(datetime)
    } else {
        let (_, date) = Date::parse(&*input.value)?;
        DateOrDateTime::Date(date)
    })
}

/// Expect a datetime value, unless there is the parameter VALUE=DATE, in which
/// case date instead
pub fn parse_date_or_datetime_list(input: &mut Line<'_>) -> Result<VecOne<DateOrDateTime>> {
    let is_datetime = input
        .params
        .take(&VALUE_PARAM)
        .map(|v| -> Result<_> {
            let v = v.get_single()?;
            Ok(match &*v {
                "DATE-TIME" => true,
                "DATE" => false,
                other => bail!("unexpected recurrance id VALUE param {other}"),
            })
        })
        .transpose()?
        .unwrap_or(true);

    let input = &*input.value;
    Ok(if is_datetime {
        let (mut input, datetime) = DateTime::parse(input)?;
        let mut output = VecOne::new(DateOrDateTime::DateTime(datetime));
        while let Ok((i, _)) = tag(",")(input) {
            let (i, date) = DateTime::parse(i)?;
            input = i;
            output.push(DateOrDateTime::DateTime(date));
        }
        output
    } else {
        let (mut input, date) = Date::parse(input)?;
        let mut output = VecOne::new(DateOrDateTime::Date(date));
        while let Ok((i, _)) = tag(",")(input) {
            let (i, date) = Date::parse(i)?;
            input = i;
            output.push(DateOrDateTime::Date(date));
        }
        output
    })
}

#[derive(Debug, Error)]
#[error("expected `{tag}`")]
pub struct TagError {
    tag: &'static str,
}

pub fn tag(tag: &'static str) -> impl FnMut(&str) -> Result<(&str, &str), TagError> {
    move |input| {
        if let Some(rest) = input.strip_prefix(tag) {
            Ok((rest, tag))
        } else {
            Err(TagError { tag })
        }
    }
}

/// Returns Ok(None) if no digits, Err if overflow
pub fn parse_u32(input: &str) -> Result<Option<(&str, u32)>> {
    fn int(ch: char) -> u32 {
        ch as u32 - '0' as u32
    }
    let Some((mut input, number)) = take_digit(input) else {
        return Ok(None);
    };
    let mut number = int(number);
    while let Some((i, ch)) = take_digit(input) {
        input = i;
        let Some(n) = number.checked_mul(10).and_then(|n| n.checked_add(int(ch))) else {
            bail!("overflow");
        };
        number = n;
    }
    Ok(Some((input, number)))
}

fn take_digit(input: &str) -> Option<(&str, char)> {
    let mut iter = input.chars();
    let ch = iter.next().filter(|ch| ch.is_ascii_digit())?;
    Some((iter.as_str(), ch))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    #[test]
    fn split_once() {
        let input = "first;second;";

        let (first, rest) = super::split_once(Cow::Borrowed(input), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::split_once(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        let (first, rest) = super::split_once(Cow::Borrowed(input), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");

        let (first, rest) = super::split_once(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");
    }

    #[test]
    fn try_split_once() {
        let input = "first;second;";

        let (first, rest) = super::try_split_once(Cow::Borrowed(input), ';').unwrap();
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::try_split_once(Cow::Owned(input.to_string()), ';').unwrap();
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        assert!(super::try_split_once(Cow::Borrowed(input), ';').is_err());
        assert!(super::try_split_once(Cow::Owned(input.to_string()), ';').is_err());
    }

    #[test]
    fn split_once_outside_quotes() {
        let input = "first;second;";

        let (first, rest) = super::split_once_outside_quotes(Cow::Borrowed(input), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::split_once_outside_quotes(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        let (first, rest) = super::split_once_outside_quotes(Cow::Borrowed(input), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");

        let (first, rest) = super::split_once_outside_quotes(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");
    }

    #[test]
    fn pop_front_bytes() {
        let mut input = "   test".to_string();
        super::pop_front_bytes(&mut input, 3);
        assert_eq!(input, "test");
    }
}
