use std::{borrow::Cow, fmt};

use anyhow::bail;
use base64::{Engine, prelude::BASE64_STANDARD};

use crate::{Result, values::Uri};

pub enum Attachment<'src> {
    Uri(Uri<'src>),
    /// Currently data is eagerly parsed.
    ///
    /// If you have a use-case for lazy parsing please raise an issue.
    Blob(Vec<u8>),
}

impl<'src> fmt::Debug for Attachment<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Attachment::Uri(uri) => write!(f, "{uri:?}"),
            Attachment::Blob(_) => f.write_str("Blob"),
        }
    }
}

impl<'src> Attachment<'src> {
    pub fn data(&self) -> Result<Vec<u8>> {
        let Self::Blob(data) = self else {
            bail!("can only extract data if local");
        };
        Ok(BASE64_STANDARD.decode(&**data)?)
    }

    pub fn data_is_local(&self) -> bool {
        matches!(self, Self::Blob(_))
    }

    pub(crate) fn parse_blob(input: Cow<'src, str>) -> Result<Self> {
        Ok(Self::Blob(BASE64_STANDARD.decode(&*input)?))
    }

    pub(crate) fn parse_uri(input: Cow<'src, str>) -> Result<Self> {
        Ok(Self::Uri(input.try_into()?))
    }
}
