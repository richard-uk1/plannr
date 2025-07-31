//! Turns text input into lines

use std::{borrow::Cow, str::Chars};

use anyhow::{Context, anyhow, bail};

use crate::line_iter::LineIter;

#[derive(Debug)]
pub enum ICalLine {
    Begin(String),
    End(String),
    ProdID(String),
    Version(String),
    CalScale(String),
    Tzid(String),
    TzOffsetFrom(String),
    TzOffsetTo(String),
    TzName(String),
    DtStart(String),
    DtEnd(String),
    RRule(String),
    /// An unrecognised extension
    Extension {
        name: String,
        value: String,
    },
}

impl<'src> TryFrom<&'src str> for ICalLine {
    type Error = anyhow::Error;
    fn try_from(input: &'src str) -> Result<Self, Self::Error> {
        let line = parse_line(input)?;
        let Name::Iana(key) = line.name else {
            todo!("extensions");
        };
        if key.eq_ignore_ascii_case("BEGIN") {
            Ok(ICalLine::Begin(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("END") {
            Ok(ICalLine::End(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("PRODID") {
            Ok(ICalLine::ProdID(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("VERSION") {
            Ok(ICalLine::Version(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("CALSCALE") {
            Ok(ICalLine::CalScale(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("TZID") {
            Ok(ICalLine::Tzid(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("TZOFFSETFROM") {
            Ok(ICalLine::TzOffsetFrom(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("TZOFFSETTO") {
            Ok(ICalLine::TzOffsetTo(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("TZNAME") {
            Ok(ICalLine::TzName(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("DTSTART") {
            Ok(ICalLine::DtStart(line.value.to_owned()))
        } else if key.eq_ignore_ascii_case("RRULE") {
            Ok(ICalLine::RRule(line.value.to_owned()))
        } else if key.starts_with("X-") {
            // TODO should we be case-insensitive here?
            Ok(ICalLine::Extension {
                name: key.to_owned(),
                value: line.value.to_owned(),
            })
        } else {
            bail!("unexpected iCal key `{key}`")
        }
    }
}

pub fn print_lines(input: &str) {
    for line in LineIter::new(input) {
        println!("{:#?}", parse_line(&*line).unwrap());
    }
}

/// Parse the next line off the input
pub fn line_iter(input: &str) -> impl Iterator<Item = anyhow::Result<ICalLine>> {
    LineIter::new(input).map(|line| ICalLine::try_from(&*line))
}

#[derive(Debug, PartialEq)]
struct Line<'src> {
    name: Name<'src>,
    params: Vec<Param<'src>>,
    value: &'src str,
}

fn parse_line<'src>(input: &'src str) -> anyhow::Result<Line<'src>> {
    // no escaping in name so easier to parse
    let mut name_iter = input.splitn(2, ":");
    let Some((prefix, value)) = input.split_once(':') else {
        bail!("malformed icalendar line: {}", input);
    };
    let Some((name, params_str)) = prefix.split_once(';') else {
        let name = parse_name(prefix)?;
        return Ok(Line {
            name,
            params: vec![],
            value,
        });
    };

    let name = parse_name(name)?;

    let mut params = vec![];
    let mut loop_rest = params_str;
    while !loop_rest.is_empty() {
        // slightly inefficient to look ahead for ';' I think but much simpler and easier to program.
        let (first_param, rest) = split_once_outside_quotes(';', loop_rest);
        params.push(parse_param(first_param)?);
        loop_rest = rest;
    }
    Ok(Line {
        name,
        params,
        value,
    })
}

#[derive(Debug, PartialEq)]
struct Param<'src> {
    name: Name<'src>,
    // values are comma-separated list
    values: Vec<&'src str>,
}

fn parse_param(input: &str) -> anyhow::Result<Param<'_>> {
    let Some((name, rest)) = input.split_once('=') else {
        bail!("invalid parameter `{input}`: no '='");
    };

    let name = parse_name(name)?;
    let mut values = vec![];
    let mut rest_loop = rest;
    while !rest_loop.is_empty() {
        let (next_param, rest) = split_once_outside_quotes(',', rest_loop);
        // we're pretty lax here but it will work on well-formed input and not do anything too stupid
        // on malformed input
        values.push(next_param.trim_matches('"'));
        rest_loop = rest;
    }
    Ok(Param { name, values })
}

#[derive(Debug, PartialEq)]
enum Name<'src> {
    XName(XName<'src>),
    Iana(&'src str),
}

fn parse_name<'src>(input: &'src str) -> anyhow::Result<Name<'src>> {
    let mut chars = input.chars();
    if matches!(chars.next(), Some('X')) && matches!(chars.next(), Some('-')) {
        let x_name = parse_x_name(&input[2..]);
        Ok(Name::XName(x_name))
    } else {
        // check skipped for speed
        //parse_iana_token(input)
        Ok(Name::Iana(input))
    }
}

#[derive(Debug, PartialEq)]
struct XName<'src> {
    /// 3-character ascii alphanumeric
    vendor: Option<[u8; 3]>,
    value: &'src str,
}

/// Currently we don't check that the value (after the vendor ID) satisfies `[0-9a-zA-z-]*`
fn parse_x_name<'src>(input: &'src str) -> XName<'src> {
    let mut chars = input.chars();
    if matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && matches!(chars.next(), Some('-'))
    {
        // Panic: input has at at least 4 bytes so no out-of-bounds possible
        let bytes = input.as_bytes();
        let vendor = [bytes[0], bytes[1], bytes[2]];
        XName {
            vendor: Some(vendor),
            value: chars.as_str(),
        }
    } else {
        XName {
            vendor: None,
            value: input,
        }
    }
}

/// parse `iana-token` (anything matching BNF not just registered tokens)
fn parse_iana_token(input: &str) -> anyhow::Result<()> {
    if input.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
        Ok(())
    } else {
        // we use the term 'name' because it is easier to understand
        Err(anyhow!("{input} is not a valid name"))
    }
}

fn split_once_or_all(ch: char, input: &str) -> (&str, &str) {
    match input.split_once(ch) {
        Some((first, rest)) => (first, rest),
        None => (input, ""),
    }
}

/// Split on the given character, or return everything in `.0` if it isn't
/// found.
///
/// This function will only split outside quoted strings (i.e. when the
/// number of seen quotes is odd).
fn split_once_outside_quotes(test_ch: char, input: &str) -> (&str, &str) {
    let mut chars = input.chars();
    let mut inside_quote = false;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            inside_quote = !inside_quote;
        } else if ch == test_ch && !inside_quote {
            let rest = chars.as_str();
            let first = &input[..input.len() - rest.len() - test_ch.len_utf8()];
            return (first, rest);
        }
    }
    (input, "")
}

#[cfg(test)]
mod tests {
    use super::{Line, Name, Param, XName};

    #[test]
    fn parse_line() {
        let input = "param-name;val1=a,b;X-aaa-val2=\"c\",d-d:actual value";
        let output = super::parse_line(input).unwrap();

        assert_eq!(
            output,
            Line {
                name: Name::Iana("param-name"),
                params: vec![
                    Param {
                        name: Name::Iana("val1"),
                        values: vec!["a", "b"]
                    },
                    Param {
                        name: Name::XName(XName {
                            vendor: Some([b'a', b'a', b'a']),
                            value: "val2"
                        }),
                        values: vec!["c", "d-d"]
                    }
                ],
                value: "actual value"
            }
        )
    }
}
