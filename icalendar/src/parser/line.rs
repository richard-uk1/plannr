use std::borrow::Cow;

use anyhow::bail;

use crate::{
    parser::{
        ParamMap,
        helpers::{split_once, split_once_outside_quotes, try_split_once},
    },
    types::Name,
};

/// Lines are split on `\r\n`, but single lines can also be split with `\r\n ` (extra space) between them.
///
/// This iterator returns 'unfolded' lines
pub struct LineIter<'src> {
    input: &'src str,
}

impl<'src> LineIter<'src> {
    pub fn new(input: &'src str) -> Self {
        Self { input }
    }
}

impl<'src> Iterator for LineIter<'src> {
    type Item = Cow<'src, str>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.input.split("\r\n");
        // Unwrap: splitn iterator always succeeds once.
        let first = iter.next().unwrap();
        let second = iter.next();
        let Some(second) = second else {
            match first {
                "" => return None,
                line => {
                    // last line
                    self.input = "";
                    return Some(Cow::Borrowed(line));
                }
            }
        };
        if !second.starts_with(" ") {
            // skip first line and `\r\n` - we will be on a char boundary
            self.input = &self.input[first.len() + 2..];
            return Some(Cow::Borrowed(first));
        }

        // we have at least 1 extension line
        let mut output = first.to_owned();
        // first char is space, we are on a char boundary
        output.push_str(&second[1..]);
        let mut len = first.len() + 2 + second.len();
        while let Some(next) = iter.next() {
            if next.starts_with(" ") {
                // first char is space, we are on a char boundary
                output.push_str(&next[1..]);
                len += next.len() + 2;
            } else {
                // `next` is following line
                // add 2 for "\r\n"
                len += 2;
                self.input = &self.input[len..];
                return Some(Cow::Owned(output));
            }
        }
        // we got to the end of the iterator
        self.input = "";
        Some(Cow::Owned(output))
    }
}

/// Parsed input line
///
/// Intermediate stage in calendar parsing
#[derive(Debug, PartialEq)]
pub struct Line<'src> {
    pub name: Name<'src>,
    pub params: ParamMap<'src>,
    pub value: Cow<'src, str>,
}

impl<'src> Line<'src> {
    pub(crate) fn parse(input: impl Into<Cow<'src, str>>) -> anyhow::Result<Self> {
        let input = input.into();

        // no escaping in name so easier to parse
        let (prefix, value) = match try_split_once(input, ':') {
            Ok(v) => v,
            Err(input) => bail!("malformed icalendar line: {input}"),
        };
        let (name, params_str) = split_once(prefix, ';');

        let name = Name::parse(name)?;

        let mut params = ParamMap::default();
        let mut loop_rest = params_str;
        while !loop_rest.is_empty() {
            // slightly inefficient to look ahead for ';' I think but much simpler and easier to program.
            let (param, rest) = split_once_outside_quotes(loop_rest, ';');
            params.parse_param(param)?;
            loop_rest = rest;
        }
        Ok(Line {
            name,
            params,
            value,
        })
    }

    pub fn first_iana_param(&self) -> Option<&Cow<'src, str>> {
        self.params.iana().next().map(|(v, _)| v)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::{
        parser::{ParamMap, line::Line},
        types::{Name, XName},
    };

    macro_rules! gen_test {
        ($name:ident : $input:expr => $output:expr) => {
            #[test]
            fn $name() {
                let input = $input;
                let output: Vec<_> = super::LineIter::new(input).collect();
                assert_eq!(output, $output)
            }
        };
    }
    gen_test!(single_line: "SIMPLE:A simple line" => ["SIMPLE:A simple line"]);
    gen_test!(two_lines: "Two\r\nlines" => ["Two", "lines"]);
    gen_test!(single_line_with_newline: "Single\nline" => ["Single\nline"]);
    gen_test!(continue_line: "Line with\r\n  continuation" => ["Line with continuation"]);
    gen_test!(
        mult_continue_line:
        "First line\r\n  with continuation\r\nSecond line \r\nThird line wi\r\n th continuation" =>
        [
            "First line with continuation",
            "Second line ",
            "Third line with continuation"
        ]
    );

    #[test]
    fn parse_line() {
        let input = "param-name;val1=a,b;X-aaa-val2=\"c\",d-d:actual value";
        let expected = Line {
            name: Name::Iana(Cow::Borrowed("param-name")),
            params: ParamMap::default()
                .with_values(
                    Name::Iana(Cow::Borrowed("val1")),
                    [Cow::Borrowed("a"), Cow::Borrowed("b")],
                )
                .with_values(
                    Name::XName(XName {
                        vendor: Some([b'a', b'a', b'a']),
                        value: Cow::Borrowed("val2"),
                    }),
                    [Cow::Borrowed("c"), Cow::Borrowed("d-d")],
                ),
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
                params: ParamMap::default(),
                value: Cow::Borrowed("actual value")
            }
        )
    }
}
