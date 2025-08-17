//! Generic helpers for parsing

use std::usize;

use smallvec::SmallVec;

use crate::parser::{Parser, error::ParserError};

struct TakeWhileMN<T> {
    inner_parser: T,
    min: usize,
    max: usize,
}

impl<T> TakeWhileMN<T> {
    fn new(inner_parser: T) -> Self {
        Self {
            inner_parser,
            min: 0,
            max: usize::MAX,
        }
    }

    /// Is a number of matches valid
    fn in_range(&self, val: usize) -> bool {
        self.min <= val && val <= self.max
    }
}

impl<T> Parser for TakeWhileMN<T>
where
    T: Parser,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    type Output<'src> = SmallVec<[T::Output<'src>; 3]>;
    type Error = ParserError;

    fn parse<'src>(
        &self,
        mut input: &'src str,
    ) -> Result<(&'src str, Self::Output<'src>), Self::Error> {
        let mut output = SmallVec::new();
        while !input.is_empty() {
            if output.len() >= self.max {
                // we've parsed the max allowed so we're done
                break;
            }
            match self.inner_parser.parse(input) {
                Ok((i, val)) => {
                    output.push(val);
                    input = i
                }
                Err(e) => {
                    return if self.in_range(output.len()) {
                        Ok((input, output))
                    } else {
                        Err(ParserError::take_while_m_n(self.min, self.max, e))
                    };
                }
            }
        }
        Ok((input, output))
    }
}

pub(crate) struct Tag {
    tag: &'static str,
}

impl Parser for Tag {
    type Output<'src> = &'src str;
    type Error = TagError;

    fn parse<'src>(
        &self,
        input: &'src str,
    ) -> Result<(&'src str, Self::Output<'src>), Self::Error> {
        let rest = input
            .strip_prefix(self.tag)
            .ok_or(TagError { expected: self.tag })?;
        Ok((rest, self.tag))
    }
}

pub(crate) struct TagError {
    expected: &'static str,
}

impl From<TagError> for ParserError {
    fn from(value: TagError) -> Self {
        Self::tag(value.expected)
    }
}
