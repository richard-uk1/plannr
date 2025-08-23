use std::{
    borrow::Cow,
    fmt,
    ops::{Index, IndexMut},
};

use crate::Result;

/// A vector with at least 1 element
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecOne<T> {
    pub first: T,
    pub rest: Vec<T>,
}

impl<T> VecOne<T> {
    pub(crate) fn parse_comma_separated<E>(
        input: &str,
        mut t_parser: impl FnMut(&str) -> Result<(&str, T), E>,
    ) -> Result<(&str, Self), E> {
        let (mut input, first) = t_parser(input)?;
        let mut rest = vec![];
        while let Some(i) = input.strip_prefix(',') {
            let (i, val) = t_parser(i)?;
            rest.push(val);
            input = i;
        }
        Ok((input, Self { first, rest }))
    }
}

impl<T> Index<usize> for VecOne<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.first,
            n => &self.rest[n - 1],
        }
    }
}

impl<T> IndexMut<usize> for VecOne<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.first,
            n => &mut self.rest[n - 1],
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for VecOne<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entry(&self.first)
            .entries(&self.rest)
            .finish()
    }
}

// Internal helpers
impl<'src> VecOne<Cow<'src, str>> {
    pub(crate) fn start_new(&mut self) {
        self.rest.push(Cow::Borrowed(""));
    }
    pub(crate) fn current(&mut self) -> &mut Cow<'src, str> {
        self.rest.last_mut().unwrap_or(&mut self.first)
    }
    pub(crate) fn add_to_current(
        &mut self,
        input: &'src str,
        current_start: usize,
        idx: usize,
        ch: char,
    ) {
        match self.current() {
            Cow::Borrowed(slice) => *slice = &input[current_start..idx + ch.len_utf8()],
            Cow::Owned(string) => string.push(ch),
        }
    }
}
