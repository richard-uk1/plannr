//! Schema for recognised parameters
//!
//! TODO Cowify

use core::fmt;
use std::{borrow::Cow, error::Error as StdError};

use anyhow::{anyhow, bail};
use oxilangtag::LanguageTag;

use crate::{
    Result,
    parser::helpers::check_param_text,
    types::{Name, VecOne},
    values::{CalendarUserAddress, Uri},
};
// NOTE: No double quotes in any param values. If the value contains
// ";", ":" or ",", it should be surrounded in double quotes.

/// General error type for single params
#[derive(Debug)]
pub enum SingleParamError<E> {
    SingleParam,
    Inner(E),
}

impl<E: fmt::Display> fmt::Display for SingleParamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleParamError::SingleParam => f.write_str("expected a single parameter"),
            SingleParamError::Inner(_) => todo!(),
        }
    }
}

impl<E: StdError> StdError for SingleParamError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            SingleParamError::SingleParam => None,
            SingleParamError::Inner(inner) => inner.source(),
        }
    }
}

impl<E> From<E> for SingleParamError<E> {
    fn from(value: E) -> Self {
        Self::Inner(value)
    }
}

pub(crate) trait ParseParam<'src>: Sized {
    const PARAM_NAME: Name<'static>;
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self>;
}

// ALTREP

#[derive(Debug)]
pub struct AlternativeTextRepresentation<'src>(pub Uri<'src>);

impl<'src> ParseParam<'src> for AlternativeTextRepresentation<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("ALTREP");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(AlternativeTextRepresentation(Uri::try_from(input)?))
    }
}

impl fmt::Display for AlternativeTextRepresentation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
    }
}

// CN

#[derive(Debug)]
pub(crate) struct CommonName<'src>(pub Cow<'src, str>);

impl<'src> ParseParam<'src> for CommonName<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("CN");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(Self(input))
    }
}

impl fmt::Display for CommonName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Would be faster to always quote.
        // Same for others below
        if self.0.contains([':', ';', ',']) {
            write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
        } else {
            write!(f, "{}={}", Self::PARAM_NAME, self.0)
        }
    }
}

// CUTYPE

#[derive(Debug, Default)]
pub enum CalendarUserType<'src> {
    #[default]
    Individual,
    Group,
    Resource,
    Room,
    Unknown,
    Name(Name<'src>),
}
impl<'src> ParseParam<'src> for CalendarUserType<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("CUTYPE");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(match &*input {
            "INDIVIDUAL" => Self::Individual,
            "GROUP" => Self::Group,
            "RESOURCE" => Self::Resource,
            "ROOM" => Self::Room,
            "UNKNOWN" => Self::Unknown,
            _ => Self::Name(Name::parse(input)?),
        })
    }
}

impl fmt::Display for CalendarUserType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            CalendarUserType::Individual => write!(f, "INDIVIDUAL"),
            CalendarUserType::Group => write!(f, "GROUP"),
            CalendarUserType::Resource => write!(f, "RESOURCE"),
            CalendarUserType::Room => write!(f, "ROOM"),
            CalendarUserType::Unknown => write!(f, "UNKNOWN"),
            CalendarUserType::Name(name) => {
                //name cannot have chars like ',' in it so don't quote
                write!(f, "{name}")
            }
        }
    }
}

// DELEGATED-FROM

pub(crate) struct Delegators<'src>(pub VecOne<CalendarUserAddress<'src>>);

impl<'src> ParseParam<'src> for Delegators<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("DELEGATED-FROM");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let first = CalendarUserAddress::try_from(input.first)?;
        let rest = input
            .rest
            .into_iter()
            .map(|val| CalendarUserAddress::try_from(val))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Delegators(VecOne::from_parts(first, rest)))
    }
}

impl<'src> fmt::Display for Delegators<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0.first)?;
        for val in &self.0.rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// DELEGATED-TO

pub(crate) struct Delegatees<'src>(pub VecOne<CalendarUserAddress<'src>>);

impl<'src> ParseParam<'src> for Delegatees<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("DELEGATED-TO");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        Ok(Delegatees(
            input.map(|text| Ok(CalendarUserAddress::try_from(text)?))?,
        ))
    }
}

impl<'src> fmt::Display for Delegatees<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0.first)?;
        for val in &self.0.rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// DIR

#[derive(Debug)]
pub struct DirectoryEntryReference<'src>(pub Uri<'src>);

impl<'src> ParseParam<'src> for DirectoryEntryReference<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("DIR");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let value = input.get_single()?;
        Ok(DirectoryEntryReference(Uri::try_from(value)?))
    }
}

impl<'src> fmt::Display for DirectoryEntryReference<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
    }
}

// ENCODING

// Must be set to BASE64 with param `VALUE=BINARY`
pub enum Encoding {
    _8Bit,
    Base64,
}

impl Encoding {
    pub const PARAM_NAME: &'static str = "ENCODING";
    pub fn parse_value(
        first: &str,
        rest: &[&str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "8BIT" => Self::_8Bit,
            "BASE64" => Self::Base64,
            _other => {
                return Err(SingleParamError::Inner(anyhow!(
                    "expected \"8BIT\" or \"BASE64\""
                )));
            }
        })
    }
    fn as_str(&self) -> &'static str {
        match self {
            Encoding::_8Bit => "8BIT",
            Encoding::Base64 => "BASE64",
        }
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, self.as_str())
    }
}

// FMTTYPE

#[derive(Debug)]
pub struct FormatType<'src>(Name<'src>);

impl<'src> ParseParam<'src> for FormatType<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("FMTTYPE");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(FormatType(Name::parse(input)?))
    }
}

impl<'src> fmt::Display for FormatType<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, &self.0)
    }
}

// FBTYPE

pub enum FreeBusyTimeType<'src> {
    Free,
    Busy,
    BusyUnavailable,
    BusyTentative,
    Name(Name<'src>),
}

impl<'src> FreeBusyTimeType<'src> {
    pub const PARAM_NAME: &'static str = "FBTYPE";
    pub fn parse_value(first: &'src str, rest: &[&'src str]) -> anyhow::Result<Self> {
        if !rest.is_empty() {
            bail!("expected single mediatype");
        }
        Ok(match first {
            "FREE" => Self::Free,
            "BUSY" => Self::Busy,
            "BUSY-UNAVAILABLE" => Self::BusyUnavailable,
            "BUSY-TENTATIVE" => Self::BusyTentative,
            other => Self::Name(Name::parse(Cow::Borrowed(other))?),
        })
    }
}

impl<'src> fmt::Display for FreeBusyTimeType<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            FreeBusyTimeType::Free => f.write_str("FREE"),
            FreeBusyTimeType::Busy => f.write_str("BUSY"),
            FreeBusyTimeType::BusyUnavailable => f.write_str("BUSY-UNAVAILABLE"),
            FreeBusyTimeType::BusyTentative => f.write_str("BUSY-TENTATIVE"),
            FreeBusyTimeType::Name(name) => fmt::Display::fmt(name, f),
        }
    }
}

// LANGUAGE

#[derive(Debug)]
pub struct Language<'src>(pub LanguageTag<Cow<'src, str>>);

impl<'src> ParseParam<'src> for Language<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("LANGUAGE");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let value = input.get_single()?;
        Ok(Language(LanguageTag::parse(value)?))
    }
}

impl<'src> fmt::Display for Language<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, &self.0)
    }
}

// MEMBER

pub(crate) struct GroupOrListMember<'src>(pub VecOne<CalendarUserAddress<'src>>);

impl<'src> ParseParam<'src> for GroupOrListMember<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("MEMBER");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> anyhow::Result<Self> {
        Ok(GroupOrListMember(
            input.map(|v| Ok(CalendarUserAddress::try_from(v)?))?,
        ))
    }
}

impl<'src> fmt::Display for GroupOrListMember<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (first, rest) = self.0.iter();
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, first)?;
        for val in rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// PARTSTAT

/// Participation status
///
/// Expected one of 'needs-action', 'accepted', 'declined' or 'delegated' for event,
/// any for todo, and one of 'needs-action', 'accepted', 'declined' for participant
/// status, but any text that would be a valid [`Name`] is valid.
#[derive(Debug)]
pub enum ParticipationStatus<'src> {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
    Delegated,
    Completed,
    InProcess,
    Name(Name<'src>),
}

impl<'src> ParseParam<'src> for ParticipationStatus<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("PARTSTAT");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(match &*input {
            "NEEDS-ACTION" => Self::NeedsAction,
            "ACCEPTED" => Self::Accepted,
            "DECLINED" => Self::Declined,
            "TENTATIVE" => Self::Tentative,
            "DELEGATED" => Self::Delegated,
            "COMPLETED" => Self::Completed,
            "IN-PROCESS" => Self::InProcess,
            _ => Self::Name(Name::parse(input)?),
        })
    }
}

impl<'src> Default for ParticipationStatus<'src> {
    fn default() -> Self {
        Self::NeedsAction
    }
}

impl<'src> fmt::Display for ParticipationStatus<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            ParticipationStatus::NeedsAction => f.write_str("NEEDS-ACTION"),
            ParticipationStatus::Accepted => f.write_str("ACCEPTED"),
            ParticipationStatus::Declined => f.write_str("DELCINED"),
            ParticipationStatus::Tentative => f.write_str("TENTATIVE"),
            ParticipationStatus::Delegated => f.write_str("DELEGATED"),
            ParticipationStatus::Completed => f.write_str("COMPLETED"),
            ParticipationStatus::InProcess => f.write_str("IN-PROCESS"),
            ParticipationStatus::Name(name) => fmt::Display::fmt(name, f),
        }
    }
}

// RANGE

#[derive(Debug)]
pub enum Range {
    ThisAndPrior,
    ThisAndFuture,
}

impl<'src> ParseParam<'src> for Range {
    const PARAM_NAME: Name<'static> = Name::iana("RANGE");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(match &*input {
            "THISANDPRIOR" => Self::ThisAndPrior,
            "THISANDFUTURE" => Self::ThisAndFuture,
            other => {
                return Err(anyhow!(
                    "expected one of THISANDPRIOR, THISANDFUTURE, found `{other}`"
                ));
            }
        })
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::ThisAndPrior => "THISANDPRIOR",
            Self::ThisAndFuture => "THISANDFUTURE",
        };
        write!(f, "{}={text}", Self::PARAM_NAME)
    }
}

// RELATED

pub enum AlarmTriggerRelationship {
    Start,
    End,
}

impl AlarmTriggerRelationship {
    pub const PARAM_NAME: &'static str = "RELATED";
    pub fn parse_value(
        first: &str,
        rest: &[&str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "START" => Self::Start,
            "END" => Self::End,
            other => return Err(anyhow!("expected `START` or `END`, found {other}").into()),
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AlarmTriggerRelationship::Start => "START",
            AlarmTriggerRelationship::End => "END",
        }
    }
}

impl Default for AlarmTriggerRelationship {
    fn default() -> Self {
        Self::Start
    }
}

impl fmt::Display for AlarmTriggerRelationship {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, self.as_str())
    }
}

// RELTYPE

pub enum RelationshipType<'src> {
    Parent,
    Child,
    Sibling,
    Name(Name<'src>),
}

impl<'src> RelationshipType<'src> {
    pub const PARAM_NAME: &'static str = "RELTYPE";
    pub fn parse_value(
        first: &'src str,
        rest: &[&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "PARENT" => Self::Parent,
            "CHILD" => Self::Child,
            "SIBLING" => Self::Sibling,
            other => Self::Name(Name::parse(Cow::Borrowed(other))?),
        })
    }
}

impl<'src> Default for RelationshipType<'src> {
    fn default() -> Self {
        Self::Parent
    }
}

impl<'src> fmt::Display for RelationshipType<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            RelationshipType::Parent => f.write_str("PARENT"),
            RelationshipType::Child => f.write_str("CHILD"),
            RelationshipType::Sibling => f.write_str("SIBLING"),
            RelationshipType::Name(name) => fmt::Display::fmt(name, f),
        }
    }
}

// ROLE

/// Specifies the participation role for the calendar user specified
/// by the property in the group schedule calendar component.
#[derive(Debug)]
pub enum ParticipationRole<'src> {
    /// Indicates the chair of the calendar entry
    Chair,
    /// Indicates a participant whose participation is required
    ReqParticipant,
    /// Indicates a participant whose participation is optional
    OptParticipant,
    /// Indicates a participant who is copied for information
    NonParticipant,
    /// Other iana or experimental role
    Name(Name<'src>),
}

impl<'src> ParseParam<'src> for ParticipationRole<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("ROLE");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(match &*input {
            "CHAIR" => Self::Chair,
            "REQ-PARTICIPANT" => Self::ReqParticipant,
            "OPT-PARTICIPANT" => Self::OptParticipant,
            "NON-PARTICIPANT" => Self::NonParticipant,
            _ => Self::Name(Name::parse(input)?),
        })
    }
}

impl<'src> Default for ParticipationRole<'src> {
    fn default() -> Self {
        Self::ReqParticipant
    }
}

impl<'src> fmt::Display for ParticipationRole<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            ParticipationRole::Chair => f.write_str("CHAIR"),
            ParticipationRole::ReqParticipant => f.write_str("REQ-PARTICIPANT"),
            ParticipationRole::OptParticipant => f.write_str("OPT-PARTICIPANT"),
            ParticipationRole::NonParticipant => f.write_str("NON-PARTICIPANT"),
            ParticipationRole::Name(name) => fmt::Display::fmt(name, f),
        }
    }
}

// RSVP

/// To specify whether there is an expectation of a favor of a reply from the
/// calendar user specified by the property value.
#[derive(Debug)]
pub enum RsvpExpectation {
    True,
    False,
}

impl<'src> ParseParam<'src> for RsvpExpectation {
    const PARAM_NAME: Name<'static> = Name::iana("RSVP");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        Ok(match &*input {
            "TRUE" => Self::True,
            "FALSE" => Self::False,
            other => return Err(anyhow!("expected `TRUE` or `FALSE`, found {other}")),
        })
    }
}

impl RsvpExpectation {
    pub fn as_str(&self) -> &'static str {
        match self {
            RsvpExpectation::True => "TRUE",
            RsvpExpectation::False => "FALSE",
        }
    }
}

impl Default for RsvpExpectation {
    fn default() -> Self {
        Self::False
    }
}

impl fmt::Display for RsvpExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, self.as_str())
    }
}

// SENT-BY

#[derive(Debug)]
pub struct SentBy<'src>(pub CalendarUserAddress<'src>);

impl<'src> ParseParam<'src> for SentBy<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("SENT-BY");

    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let value = input.get_single()?;
        Ok(SentBy(value.try_into()?))
    }
}

impl<'src> fmt::Display for SentBy<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
    }
}

// TZID

/// Timezone is not checked against database, just validated.
#[derive(Debug)]
pub struct TimeZoneIdentifier<'src> {
    prefix: bool,
    value: Cow<'src, str>,
}

impl<'src> ParseParam<'src> for TimeZoneIdentifier<'src> {
    const PARAM_NAME: Name<'static> = Name::iana("SENT-BY");
    fn parse_value(input: VecOne<Cow<'src, str>>) -> Result<Self> {
        let input = input.get_single()?;
        let prefix = input.starts_with('/');
        let value = if prefix {
            // Panic: first character is ASCII so 1 byte
            check_param_text(&input[1..])?;

            match input {
                Cow::Borrowed(s) => Cow::Borrowed(&s[1..]),
                Cow::Owned(mut s) => {
                    s.remove(0);
                    Cow::Owned(s)
                }
            }
        } else {
            check_param_text(&*input)?;
            input
        };
        Ok(Self { prefix, value })
    }
}

impl<'src> TimeZoneIdentifier<'src> {
    pub fn fmt_value(&self) -> impl fmt::Display {
        struct FmtValue<'a>(&'a TimeZoneIdentifier<'a>);
        impl<'a> fmt::Display for FmtValue<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "{}{}",
                    if self.0.prefix { "/" } else { "" },
                    self.0.value
                )
            }
        }
        FmtValue(self)
    }
}

impl<'src> fmt::Display for TimeZoneIdentifier<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, self.fmt_value())
    }
}

// VALUE

pub enum Value<'src> {
    Binary,
    Boolean,
    CalAddress,
    Date,
    DateTime,
    Duration,
    Float,
    Integer,
    Period,
    Recur,
    Text,
    Time,
    Uri,
    UtcOffset,
    Name(Name<'src>),
}

impl<'src> Value<'src> {
    pub const PARAM_NAME: &'static str = "ROLE";
    pub fn parse_value(
        first: &'src str,
        rest: &[&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "BINARY" => Self::Binary,
            "BOOLEAN" => Self::Boolean,
            "CAL-ADDRESS" => Self::CalAddress,
            "DATE" => Self::Date,
            "DATE-TIME" => Self::DateTime,
            "DURATION" => Self::Duration,
            "FLOAT" => Self::Float,
            "INTEGER" => Self::Integer,
            "PERIOD" => Self::Period,
            "RECUR" => Self::Recur,
            "TEXT" => Self::Text,
            "TIME" => Self::Time,
            "URI" => Self::Uri,
            "UTC-OFFSET" => Self::UtcOffset,
            other => Self::Name(Name::parse(Cow::Borrowed(other))?),
        })
    }
}

impl<'src> fmt::Display for Value<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", Self::PARAM_NAME)?;
        match self {
            Value::Binary => f.write_str("BINARY"),
            Value::Boolean => f.write_str("BOOLEAN"),
            Value::CalAddress => f.write_str("CAL-ADDRESS"),
            Value::Date => f.write_str("DATE"),
            Value::DateTime => f.write_str("DATE-TIME"),
            Value::Duration => f.write_str("DURATION"),
            Value::Float => f.write_str("FLOAT"),
            Value::Integer => f.write_str("INTEGER"),
            Value::Period => f.write_str("PERIOD"),
            Value::Recur => f.write_str("RECUR"),
            Value::Text => f.write_str("TEXT"),
            Value::Time => f.write_str("TIME"),
            Value::Uri => f.write_str("URI"),
            Value::UtcOffset => f.write_str("UTC-OFFSET"),
            Value::Name(name) => fmt::Display::fmt(name, f),
        }
    }
}
