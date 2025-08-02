//! Turns text input into lines

use core::fmt;

use anyhow::{anyhow, bail};

use crate::line_iter::LineIter;

pub fn print_lines(input: &str) {
    for line in LineIter::new(input) {
        println!("{:#?}", parse_line(&*line).unwrap());
    }
}

#[derive(Debug, PartialEq)]
struct Line<'src> {
    name: Name<'src>,
    params: Vec<Param<'src>>,
    value: &'src str,
}

fn parse_line<'src>(input: &'src str) -> anyhow::Result<Line<'src>> {
    // no escaping in name so easier to parse
    let Some((prefix, value)) = input.split_once(':') else {
        bail!("malformed icalendar line: {}", input);
    };
    let Some((name, params_str)) = prefix.split_once(';') else {
        let name = Name::parse(prefix)?;
        return Ok(Line {
            name,
            params: vec![],
            value,
        });
    };

    let name = Name::parse(name)?;

    let mut params = vec![];
    let mut loop_rest = params_str;
    while !loop_rest.is_empty() {
        // slightly inefficient to look ahead for ';' I think but much simpler and easier to program.
        let (first_param, rest) = split_once_outside_quotes(';', loop_rest);
        params.push(Param::parse(first_param)?);
        loop_rest = rest;
    }
    Ok(Line {
        name,
        params,
        value,
    })
}

#[derive(Debug, PartialEq)]
pub struct Param<'src> {
    pub name: Name<'src>,
    // values are comma-separated list, at least one
    pub first_value: &'src str,
    pub rest_values: Vec<&'src str>,
}

impl<'src> Param<'src> {
    fn parse(input: &'src str) -> anyhow::Result<Param<'src>> {
        let Some((name, rest)) = input.split_once('=') else {
            bail!("invalid parameter `{input}`: no '='");
        };

        let name = Name::parse(name)?;
        let (first_value, rest) = split_once_outside_quotes(',', rest);
        let first_value = param_value(first_value)?;

        let mut rest_loop = rest;
        let mut rest_values = vec![];
        while !rest_loop.is_empty() {
            let (next_param, rest) = split_once_outside_quotes(',', rest_loop);
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
    Iana(&'src str),
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
    pub fn parse(input: &'src str) -> anyhow::Result<Self> {
        let mut chars = input.chars();
        if matches!(chars.next(), Some('X')) && matches!(chars.next(), Some('-')) {
            let x_name = XName::parse(chars.as_str())?;
            Ok(Name::XName(x_name))
        } else {
            Ok(Name::Iana(iana_token(input)?))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct XName<'src> {
    /// 3-character ascii alphanumeric
    pub vendor: Option<[u8; 3]>,
    pub value: &'src str,
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
        fmt::Display::fmt(self.value, f)
    }
}

impl<'src> XName<'src> {
    /// Currently we don't check that the value (after the vendor ID) satisfies `[0-9a-zA-z-]*`
    fn parse(input: &'src str) -> anyhow::Result<XName<'src>> {
        let mut chars = input.chars();
        if matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            && matches!(chars.next(), Some('-'))
        {
            // Panic: input has at at least 4 bytes so no out-of-bounds possible
            let bytes = input.as_bytes();
            let vendor = [bytes[0], bytes[1], bytes[2]];
            let value = iana_token(chars.as_str())?;
            Ok(XName {
                vendor: Some(vendor),
                value,
            })
        } else {
            let value = iana_token(input)?;
            Ok(XName {
                vendor: None,
                value,
            })
        }
    }
}

/// parse `iana-token` (anything matching BNF not just registered tokens)
fn iana_token(input: &str) -> anyhow::Result<&str> {
    if input.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
        Ok(input)
    } else {
        // we use the term 'name' because it is easier to understand
        Err(anyhow!("{input} is not a valid name"))
    }
}

fn param_value(input: &str) -> anyhow::Result<&str> {
    if input.starts_with('"') {
        quoted_string(input)
    } else {
        param_text(input)
    }
}

pub(crate) fn param_text<'src>(input: &'src str) -> anyhow::Result<&'src str> {
    for ch in input.chars() {
        safe_char(ch)?;
    }
    Ok(input)
}

fn safe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        ch @ '"' | ch @ ';' | ch @ ':' | ch @ ',' => bail!("`{ch}` not allowed"),
        _ => Ok(()),
    }
}

/// Returns `input` without the start and end quotes
fn quoted_string(input: &str) -> anyhow::Result<&str> {
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
    Ok(input.trim_matches('"'))
}

fn qsafe_char(input: char) -> anyhow::Result<()> {
    match input {
        ch if ch.is_control() => bail!("control characters not allowed"),
        '"' => bail!("`\"` is not allowed"),
        _ => Ok(()),
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
                        first_value: "a",
                        rest_values: vec!["b"]
                    },
                    Param {
                        name: Name::XName(XName {
                            vendor: Some([b'a', b'a', b'a']),
                            value: "val2"
                        }),
                        first_value: "c",
                        rest_values: vec!["d-d"]
                    }
                ],
                value: "actual value"
            }
        )
    }

    #[test]
    fn parse_xtension_no_vendor() {
        let input = "X-param-name:actual value";
        let output = super::parse_line(input).unwrap();
        assert_eq!(
            output,
            Line {
                name: Name::XName(XName {
                    vendor: None,
                    value: "param-name"
                }),
                params: vec![],
                value: "actual value"
            }
        )
    }
}
