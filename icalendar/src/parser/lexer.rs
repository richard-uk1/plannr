use std::collections::VecDeque;

use crate::{
    Result,
    parser::{Line, line_iter::LineIter},
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
