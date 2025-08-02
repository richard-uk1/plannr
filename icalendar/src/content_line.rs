use anyhow::bail;

use crate::parser::Name;

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
        todo!()
    }
}
