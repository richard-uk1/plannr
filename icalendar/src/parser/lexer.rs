use std::{borrow::Cow, collections::VecDeque};

use anyhow::bail;

use crate::{
    Result,
    parser::{
        helpers::{check_iana_token, pop_front_bytes},
        line::{Line, LineIter},
    },
    types::{Name, XName},
};

/// this is kinda like a lexer so call it that, even though it's not exactly
pub struct Lexer<'src> {
    input: LineIter<'src>,
    cache: VecDeque<Line<'src>>,
}

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        Self {
            input: LineIter::new(input),
            cache: VecDeque::with_capacity(3),
        }
    }

    pub fn is_empty(&mut self) -> Result<bool> {
        Ok(self.next()?.is_none())
    }

    pub fn next(&mut self) -> Result<Option<&Line<'src>>> {
        if !self.ensure_cache()? {
            return Ok(None);
        }
        Ok(Some(self.cache.front().unwrap()))
    }

    pub fn take_next(&mut self) -> Result<Option<Line<'src>>> {
        if !self.ensure_cache()? {
            return Ok(None);
        }
        Ok(Some(self.cache.pop_front().unwrap()))
    }

    pub fn step(&mut self) {
        if self.cache.pop_front().is_none() {
            // skip an uncached line if there are no cached ones
            self.input.next();
        }
    }

    /// Assume we just began an element. Skip past the end.
    pub fn skip_current(&mut self) -> Result {
        let mut depth = 1;
        while let Some(line) = self.take_next()? {
            if &line.name == "BEGIN" {
                depth += 1;
            } else if &line.name == "END" {
                depth -= 1;
            }
            if depth == 0 {
                return Ok(());
            }
        }
        bail!("unexpected EOF");
    }

    /// Make sure there is at least one line in the cache.
    ///
    /// Returns false if it wasn't possible because the iterator is exhausted
    ///
    /// If this function errors a line will be lost
    fn ensure_cache(&mut self) -> Result<bool> {
        if self.cache.is_empty() {
            match self.input.next() {
                Some(line) => {
                    self.cache.push_back(Line::parse(line)?);
                    Ok(true)
                }
                None => Ok(false),
            }
        } else {
            Ok(true)
        }
    }
}

impl<'src> Name<'src> {
    pub fn parse(input: impl Into<Cow<'src, str>>) -> Result<Self> {
        Self::parse_inner(input.into())
    }

    fn parse_inner(input: Cow<'src, str>) -> Result<Self> {
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

impl<'src> XName<'src> {
    /// Currently we don't check that the value (after the vendor ID) satisfies `[0-9a-zA-z-]*`
    pub(crate) fn parse(input: Cow<'src, str>) -> anyhow::Result<XName<'src>> {
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
