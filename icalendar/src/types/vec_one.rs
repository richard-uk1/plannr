use std::{
    borrow::Cow,
    fmt,
    ops::{Index, IndexMut},
};

use anyhow::bail;

use crate::{Result, parser::helpers::strip_prefix};

/// A vector with at least 1 element
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecOne<T> {
    pub first: T,
    pub rest: Vec<T>,
}

impl<T> VecOne<T> {
    pub fn new(value: T) -> Self {
        Self {
            first: value,
            rest: vec![],
        }
    }

    pub fn from_parts(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }

    pub(crate) fn parse_comma_separated<'src, E>(
        input: Cow<'src, str>,
        mut t_parser: impl FnMut(Cow<'src, str>) -> Result<(Cow<'src, str>, T), E>,
    ) -> Result<(Cow<'src, str>, Self), E> {
        let (mut input, first) = t_parser(input)?;
        let mut rest = vec![];
        loop {
            match strip_prefix(input, ",") {
                Ok(i) => {
                    let (i, val) = t_parser(i)?;
                    rest.push(val);
                    input = i;
                }
                Err(i) => {
                    input = i;
                    break;
                }
            }
        }
        Ok((input, Self { first, rest }))
    }

    pub(crate) fn parse_comma_separated_borrowed<'src, E>(
        input: &'src str,
        mut t_parser: impl FnMut(&'src str) -> Result<(&'src str, T), E>,
    ) -> Result<(&'src str, Self), E> {
        let (mut input, first) = t_parser(input)?;
        let mut rest = vec![];
        loop {
            match input.strip_prefix(",") {
                Some(i) => {
                    let (i, val) = t_parser(i)?;
                    rest.push(val);
                    input = i;
                }
                None => {
                    break;
                }
            }
        }
        Ok((input, Self { first, rest }))
    }

    pub(crate) fn push(&mut self, val: T) {
        self.rest.push(val);
    }

    pub(crate) fn get_single(self) -> Result<T> {
        if !self.rest.is_empty() {
            bail!("expected 1 element, found {}", self.rest.len() + 1);
        }
        Ok(self.first)
    }

    pub(crate) fn map<T2, F>(self, mut f: F) -> Result<VecOne<T2>>
    where
        F: FnMut(T) -> Result<T2>,
    {
        Ok(VecOne {
            first: f(self.first)?,
            rest: self.rest.into_iter().map(f).collect::<Result<Vec<_>>>()?,
        })
    }

    pub(crate) fn iter(&self) -> (&T, impl Iterator<Item = &T>) {
        (&self.first, self.rest.iter())
    }
}

impl<T> Extend<T> for VecOne<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.rest.extend(iter)
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

    pub(crate) fn push_to_current(&mut self, ch: char) {
        match self.current() {
            Cow::Borrowed(_) => unreachable!(),
            Cow::Owned(string) => string.push(ch),
        }
    }
}

impl<T: fmt::Display> VecOne<T> {
    /// Display as a comma-separated list
    pub(crate) fn display(&self) -> impl fmt::Display
    where
        T: fmt::Display,
    {
        struct Display<'a, T>(&'a VecOne<T>);
        impl<'a, T: fmt::Display> fmt::Display for Display<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.first, f)?;
                for itm in &self.0.rest {
                    f.write_str(",")?;
                    fmt::Display::fmt(itm, f)?;
                }
                Ok(())
            }
        }
        Display(self)
    }
}
