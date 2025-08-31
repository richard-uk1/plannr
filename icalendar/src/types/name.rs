use std::{borrow::Cow, fmt};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Name<'src> {
    XName(XName<'src>),
    Iana(Cow<'src, str>),
}

impl<'src> Name<'src> {
    /// NOTE: this function does not check the name is a valid IANA name.
    pub(crate) const fn iana(name: &'static str) -> Self {
        let name = Cow::Borrowed(name);
        //debug_assert!(Name::parse(name).is_ok());
        Self::Iana(name)
    }

    pub(crate) fn x_unchecked(name: impl Into<XName<'src>>) -> Self {
        let name = name.into();
        Self::XName(name)
    }

    pub fn is_extension(&self) -> bool {
        matches!(self, Name::XName(_))
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::XName(xname) => fmt::Display::fmt(xname, f),
            Name::Iana(iana) => fmt::Display::fmt(iana, f),
        }
    }
}

impl fmt::Debug for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<'a> PartialEq<str> for Name<'a> {
    fn eq(&self, other: &str) -> bool {
        match self {
            Name::XName(xname) => {
                let Ok(other) = XName::parse(Cow::Borrowed(other)) else {
                    return false;
                };
                xname == &other
            }
            Name::Iana(name) => *name == other,
        }
    }
}

impl<'a> PartialEq<Name<'a>> for str {
    fn eq(&self, other: &Name<'a>) -> bool {
        other.eq(self)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XName<'src> {
    /// 3-character ascii alphanumeric
    pub vendor: Option<[u8; 3]>,
    pub value: Cow<'src, str>,
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
        fmt::Display::fmt(&self.value, f)
    }
}

impl fmt::Debug for XName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
