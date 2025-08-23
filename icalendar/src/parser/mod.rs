//! Turns text input into lines

use core::fmt;
use std::{borrow::Cow, collections::VecDeque};

use anyhow::{anyhow, bail};

mod line_iter;
use line_iter::LineIter;

mod error;
pub use error::ParserError;

mod helpers;
mod lexer;
pub(crate) use lexer::Lexer;

use crate::{
    Calendar, Result,
    parser::helpers::{pop_front_bytes, split_once, split_once_outside_quotes, try_split_once},
};

impl<'src> Calendar<'src> {
    pub(crate) fn parse(parser: &mut Lexer<'src>) -> Result<Self> {
        let Some(begin) = parser.take_next()? else {
            bail!("empty iterator: call is_empty before this function to avoid");
        };
        if !(&begin.name == "BEGIN" && begin.value == "VCALENDAR") {
            bail!("expected `BEGIN:VCALENDAR`");
        }

        let mut builder = CalendarBuilder::new();
        loop {
            let Some(next) = parser.take_next()? else {
                bail!("expected END:VCALENDAR");
            };
            if &next.name == "END" {
                if next.value != "VCALENDAR" {
                    continue;
                    bail!("expected VCALENDAR, found {}", next.value);
                }
                break;
            } else if &next.name == "PRODID" {
                builder.set_prod_id(parse_prodid(next)?)?;
            } else if &next.name == "VERSION" {
                builder.set_version(parse_version(next)?)?;
            }
        }
        Ok(builder.build()?)
    }
}

fn parse_prodid<'src>(input: Line<'src>) -> Result<Cow<'src, str>> {
    debug_assert_eq!(&input.name, "PRODID");
    if let Some(param) = input.non_x_param() {
        bail!("unexpected param {param:?}");
    }
    Ok(input.value)
}

fn parse_version<'src>(input: Line<'src>) -> Result {
    debug_assert_eq!(&input.name, "VERSION");
    if let Some(param) = input.non_x_param() {
        bail!("unexpected param {param:?}");
    }
    if input.value != "2.0" {
        bail!("only version 2.0 supported");
    }
    Ok(())
}

struct CalendarBuilder<'src> {
    prod_id: Option<Cow<'src, str>>,
    version_set: bool,
}

impl<'src> CalendarBuilder<'src> {
    fn new() -> Self {
        Self {
            prod_id: None,
            version_set: false,
        }
    }

    fn build(self) -> Result<Calendar<'src>> {
        Ok(Calendar {
            prod_id: self
                .prod_id
                .ok_or_else(|| anyhow!("PRODID not specified"))?,
        })
    }

    fn set_prod_id(&mut self, prod_id: Cow<'src, str>) -> Result {
        if self.prod_id.is_some() {
            bail!("expected 1 PRODID, found at least 2");
        }
        self.prod_id = Some(prod_id);
        Ok(())
    }

    fn set_version(&mut self, (): ()) -> Result {
        if self.version_set {
            bail!("expected 1 VERSION, found at least 2");
        }
        Ok(())
    }
}

/// for debug TODO delete me
pub fn print_lines(input: &str) {
    for line in LineIter::new(input) {
        println!("{:#?}", Line::parse(line).unwrap());
    }
}

/// Parsed input line
///
/// Intermediate stage in calendar parsing
#[derive(Debug, PartialEq)]
struct Line<'src> {
    name: Name<'src>,
    params: Vec<Param<'src>>,
    value: Cow<'src, str>,
}

impl<'src> Line<'src> {
    fn parse(input: impl Into<Cow<'src, str>>) -> anyhow::Result<Self> {
        let input = input.into();

        // no escaping in name so easier to parse
        let (prefix, value) = match try_split_once(input, ':') {
            Ok(v) => v,
            Err(input) => bail!("malformed icalendar line: {input}"),
        };
        let (name, params_str) = split_once(prefix, ';');

        let name = Name::parse(name)?;

        let mut params = vec![];
        let mut loop_rest = params_str;
        while !loop_rest.is_empty() {
            // slightly inefficient to look ahead for ';' I think but much simpler and easier to program.
            let (first_param, rest) = split_once_outside_quotes(loop_rest, ';');
            params.push(Param::parse(first_param)?);
            loop_rest = rest;
        }
        Ok(Line {
            name,
            params,
            value,
        })
    }

    pub(crate) fn non_x_param(&self) -> Option<&Param> {
        self.params.iter().find(|param| param.name.is_extension())
    }
}

#[derive(Debug, PartialEq)]
pub struct Param<'src> {
    pub name: Name<'src>,
    // values are comma-separated list, at least one
    pub first_value: Cow<'src, str>,
    pub rest_values: Vec<Cow<'src, str>>,
}

impl<'src> Param<'src> {
    fn parse(input: Cow<'src, str>) -> anyhow::Result<Param<'src>> {
        let (name, rest) = match try_split_once(input, '=') {
            Ok(v) => v,
            Err(input) => bail!("invalid parameter `{input}`: no '='"),
        };

        let name = Name::parse(name)?;
        let (first_value, rest) = split_once_outside_quotes(rest, ',');
        let first_value = param_value(first_value)?;

        let mut rest_loop = rest;
        let mut rest_values = vec![];
        while !rest_loop.is_empty() {
            let (next_param, rest) = split_once_outside_quotes(rest_loop, ',');
            // we're pretty lax here but it will work on well-formed input and not do anything too stupid
            // on malformed input

            rest_values.push(param_value(next_param)?);
            rest_loop = rest;
        }
        Ok(Param {
            name,
            first_value,
            rest_values,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Name<'src> {
    XName(XName<'src>),
    Iana(Cow<'src, str>),
}

impl<'src> Name<'src> {
    pub fn is_extension(&self) -> bool {
        matches!(self, Name::XName(_))
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::XName(xname) => fmt::Display::fmt(xname, f),
            Name::Iana(iana) => fmt::Display::fmt(iana, f),
        }
    }
}

impl<'src> Name<'src> {
    pub fn parse(input: Cow<'src, str>) -> anyhow::Result<Self> {
        match input {
            Cow::Borrowed(s) => {
                if matches!(s.get(0..2), Some("X-")) {
                    let x_name = XName::parse(Cow::Borrowed(&s[2..]))?;
                    Ok(Name::XName(x_name))
                } else {
                    check_iana_token(&s)?;
                    Ok(Name::Iana(Cow::Borrowed(s)))
                }
            }
            Cow::Owned(mut s) => {
                if matches!(s.get(0..2), Some("X-")) {
                    pop_front_bytes(&mut s, 2);
                    let x_name = XName::parse(Cow::Owned(s))?;
                    Ok(Name::XName(x_name))
                } else {
                    check_iana_token(&s)?;
                    Ok(Name::Iana(Cow::Owned(s)))
                }
            }
        }
    }
}

impl<'a> PartialEq<str> for Name<'a> {
    fn eq(&self, other: &str) -> bool {
        match self {
            Name::XName(xname) => {
                let Ok(other) = XName::parse(Cow::Borrowed(other)) else {
                    return false;
                };
                xname == &other
            }
            Name::Iana(name) => *name == other,
        }
    }
}

impl<'a> PartialEq<Name<'a>> for str {
    fn eq(&self, other: &Name<'a>) -> bool {
        other.eq(self)
    }
}

#[derive(Debug, PartialEq)]
pub struct XName<'src> {
    /// 3-character ascii alphanumeric
    pub vendor: Option<[u8; 3]>,
    pub value: Cow<'src, str>,
}

impl fmt::Display for XName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn c(input: u8) -> char {
            // Unwrap: cannot fail as vendor is alphanumeric
            char::from_u32(input.into()).unwrap()
        }
        write!(f, "X-")?;
        if let Some(vendor) = &self.vendor {
            write!(f, "{}{}{}-", c(vendor[0]), c(vendor[1]), c(vendor[2]))?;
        }
        // value is alphanumeric or '-'
        fmt::Display::fmt(&self.value, f)
    }
}

impl<'src> XName<'src> {
    /// Currently we don't check that the value (after the vendor ID) satisfies `[0-9a-zA-z-]*`
    fn parse(input: Cow<'src, str>) -> anyhow::Result<XName<'src>> {
        match input {
            Cow::Borrowed(s) => Self::parse_borrowed(s),
            Cow::Owned(s) => Self::parse_owned(s),
        }
    }

    fn parse_borrowed(input: &'src str) -> Result<XName<'src>> {
        let mut chars = input.chars();
        if matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some('-'))
        {
            // Panic: input has at at least 4 bytes so no out-of-bounds possible
            let bytes = input.as_bytes();
            let vendor = [bytes[0], bytes[1], bytes[2]];
            let value = chars.as_str();
            check_iana_token(value)?;
            Ok(XName {
                vendor: Some(vendor),
                value: Cow::Borrowed(value),
            })
        } else {
            check_iana_token(input)?;
            Ok(XName {
                vendor: None,
                value: Cow::Borrowed(input),
            })
        }
    }

    fn parse_owned(mut input: String) -> Result<XName<'static>> {
        let mut chars = input.chars();
        let (vendor, value) = if matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some('-'))
        {
            let prefix_len = input.len() - chars.as_str().len();
            let vendor = input.as_bytes();
            let vendor = [vendor[0], vendor[1], vendor[2]];
            pop_front_bytes(&mut input, prefix_len);
            (Some(vendor), input)
        } else {
            // cannot have a vendor
            (None, input)
        };
        check_iana_token(&value)?;
        Ok(XName {
            vendor,
            value: Cow::Owned(value),
        })
    }
}

/// parse `iana-token` (anything matching BNF not just registered tokens)
fn check_iana_token(input: &str) -> Result {
    if input.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
        Ok(())
    } else {
        // we use the term 'name' because it is easier to understand
        Err(anyhow!("{input} is not a valid name"))
    }
}

fn param_value<'src>(input: Cow<'src, str>) -> Result<Cow<'src, str>> {
    if input.starts_with('"') {
        quoted_string(input)
    } else {
        check_param_text(&input)?;
        Ok(input)
    }
}

pub(crate) fn check_param_text<'src>(input: &'src str) -> Result<()> {
    for ch in input.chars() {
        safe_char(ch)?;
    }
    Ok(())
}

fn safe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        ch @ '"' | ch @ ';' | ch @ ':' | ch @ ',' => bail!("`{ch}` not allowed"),
        _ => Ok(()),
    }
}

/// Returns `input` without the start and end quotes
fn quoted_string(input: Cow<'_, str>) -> anyhow::Result<Cow<'_, str>> {
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

fn qsafe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        '"' => bail!("`\"` is not allowed"),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{Line, Name, Param, XName};

    #[test]
    fn parse_line() {
        let input = "param-name;val1=a,b;X-aaa-val2=\"c\",d-d:actual value";
        let expected = Line {
            name: Name::Iana(Cow::Borrowed("param-name")),
            params: vec![
                Param {
                    name: Name::Iana(Cow::Borrowed("val1")),
                    first_value: Cow::Borrowed("a"),
                    rest_values: vec![Cow::Borrowed("b")],
                },
                Param {
                    name: Name::XName(XName {
                        vendor: Some([b'a', b'a', b'a']),
                        value: Cow::Borrowed("val2"),
                    }),
                    first_value: Cow::Borrowed("c"),
                    rest_values: vec![Cow::Borrowed("d-d")],
                },
            ],
            value: Cow::Borrowed("actual value"),
        };

        // borrowed
        let output = Line::parse(input).unwrap();
        assert_eq!(output, expected);

        // owned
        let output = Line::parse(input.to_string()).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_xtension_no_vendor() {
        let input = "X-param-name:actual value";
        let output = Line::parse(input).unwrap();
        assert_eq!(
            output,
            Line {
                name: Name::XName(XName {
                    vendor: None,
                    value: Cow::Borrowed("param-name")
                }),
                params: vec![],
                value: Cow::Borrowed("actual value")
            }
        )
    }
}
