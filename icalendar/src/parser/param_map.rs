use std::{borrow::Cow, collections::HashMap};

use anyhow::bail;

use crate::{
    Result,
    params::ParseParam,
    parser::helpers::{param_value, split_once_outside_quotes, try_split_once},
    types::{Name, VecOne, XName},
};

#[derive(Debug, Default, PartialEq)]
pub struct ParamMap<'src> {
    iana: HashMap<Cow<'src, str>, VecOne<Cow<'src, str>>>,
    extend: HashMap<XName<'src>, VecOne<Cow<'src, str>>>,
}

impl<'src> ParamMap<'src> {
    pub(crate) fn parse_param(&mut self, input: Cow<'src, str>) -> Result {
        let (name, rest) = match try_split_once(input, '=') {
            Ok(v) => v,
            Err(input) => bail!("invalid parameter `{input}`: no '='"),
        };

        let name = Name::parse(name)?;

        let mut values = vec![];
        let mut input = rest;
        while !input.is_empty() {
            let (next_param, i) = split_once_outside_quotes(input, ',');
            // we're pretty lax here but it will work on well-formed input and not do anything too stupid
            // on malformed input

            values.push(param_value(next_param)?);
            input = i;
        }

        self.add_values(name, values);
        Ok(())
    }

    pub fn push(&mut self, name: Name<'src>, value: Cow<'src, str>) -> &mut Self {
        match name {
            Name::XName(xname) => {
                if let Some(v) = self.extend.get_mut(&xname) {
                    v.push(value);
                } else {
                    self.extend.insert(xname, VecOne::new(value));
                }
            }
            Name::Iana(cow) => {
                if let Some(v) = self.iana.get_mut(&cow) {
                    v.push(value);
                } else {
                    self.iana.insert(cow, VecOne::new(value));
                }
            }
        }
        self
    }

    pub fn with_push(mut self, name: Name<'src>, value: Cow<'src, str>) -> Self {
        self.push(name, value);
        self
    }

    pub fn add_values(
        &mut self,
        name: Name<'src>,
        values: impl IntoIterator<Item = Cow<'src, str>>,
    ) -> &mut Self {
        let mut values = values.into_iter();
        let Some(first) = values.next() else {
            return self;
        };
        match name {
            Name::XName(xname) => {
                let v = if let Some(v) = self.extend.get_mut(&xname) {
                    v.push(first);
                    v
                } else {
                    self.extend.entry(xname).or_insert(VecOne::new(first))
                };
                v.extend(values);
            }
            Name::Iana(cow) => {
                let v = if let Some(v) = self.iana.get_mut(&cow) {
                    v.push(first);
                    v
                } else {
                    self.iana.entry(cow).or_insert(VecOne::new(first))
                };
                v.extend(values);
            }
        }
        self
    }

    #[cfg(test)]
    pub fn with_values(
        mut self,
        name: Name<'src>,
        values: impl IntoIterator<Item = Cow<'src, str>>,
    ) -> Self {
        self.add_values(name, values);
        self
    }

    pub fn take(&mut self, key: &Name<'src>) -> Option<VecOne<Cow<'src, str>>> {
        match key {
            Name::XName(xname) => self.extend.remove(xname),
            Name::Iana(cow) => self.iana.remove(cow),
        }
    }

    pub fn take_ty<T: ParseParam<'src>>(&mut self) -> Result<Option<T>> {
        let Some(value) = self.take(&T::PARAM_NAME) else {
            return Ok(None);
        };
        T::parse_value(value).map(Some)
    }

    pub fn iana(&self) -> impl Iterator<Item = (&Cow<'src, str>, &VecOne<Cow<'src, str>>)> {
        self.iana.iter()
    }
}
