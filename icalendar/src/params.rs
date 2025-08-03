//! Schema for recognised parameters

use core::fmt;
use std::error::Error as StdError;

use anyhow::{anyhow, bail};
use mediatype::MediaType;
use oxilangtag::LanguageTag;
use uriparse::URIError;

use crate::{
    parser::{Name, param_text},
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

// ALTREP

pub struct AlternativeTextRepresentation<'src>(pub Uri<'src>);
impl<'src> AlternativeTextRepresentation<'src> {
    pub const PARAM_NAME: &'static str = "ALTREP";

    pub fn parse_value(
        first_value: &'src str,
        rest_values: &[&'src str],
    ) -> Result<Self, SingleParamError<URIError>> {
        if !rest_values.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(AlternativeTextRepresentation(Uri::parse(first_value)?))
    }
}

impl fmt::Display for AlternativeTextRepresentation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
    }
}

// CN

pub struct CommonName<'src>(pub &'src str);
impl<'src> CommonName<'src> {
    pub const PARAM_NAME: &'static str = "CN";

    pub fn parse_value(input: &'src str) -> anyhow::Result<Self> {
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

pub enum CalendarUserType<'src> {
    Individual,
    Group,
    Resource,
    Room,
    Unknown,
    Name(Name<'src>),
}
impl<'src> CalendarUserType<'src> {
    pub const PARAM_NAME: &'static str = "CUTYPE";

    pub fn parse_value(
        first_value: &'src str,
        rest_values: &[&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest_values.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first_value {
            "INDIVIDUAL" => Self::Individual,
            "GROUP" => Self::Group,
            "RESOURCE" => Self::Resource,
            "ROOM" => Self::Room,
            "UNKNOWN" => Self::Unknown,
            other => Self::Name(Name::parse(other)?),
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

impl<'src> Default for CalendarUserType<'src> {
    fn default() -> Self {
        // matches default in specification
        Self::Individual
    }
}

// DELEGATED-FROM

pub struct Delegators<'src> {
    pub first: CalendarUserAddress<'src>,
    pub rest: Vec<CalendarUserAddress<'src>>,
}

impl<'src> Delegators<'src> {
    pub const PARAM_NAME: &'static str = "DELEGATED-FROM";
    pub fn parse_value(first: &'src str, rest: &'_ [&'src str]) -> anyhow::Result<Self> {
        let first = CalendarUserAddress::try_from(first)?;
        let rest = rest
            .iter()
            .map(|val| CalendarUserAddress::try_from(*val))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Delegators { first, rest })
    }
}

impl<'src> fmt::Display for Delegators<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.first)?;
        for val in &self.rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// DELEGATED-TO

pub struct Delegatees<'src> {
    pub first: CalendarUserAddress<'src>,
    pub rest: Vec<CalendarUserAddress<'src>>,
}

impl<'src> Delegatees<'src> {
    pub const PARAM_NAME: &'static str = "DELEGATED-TO";
    pub fn parse_value(first: &'src str, rest: &'_ [&'src str]) -> anyhow::Result<Self> {
        let first = CalendarUserAddress::try_from(first)?;
        let rest = rest
            .iter()
            .map(|val| CalendarUserAddress::try_from(*val))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Delegatees { first, rest })
    }
}

impl<'src> fmt::Display for Delegatees<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.first)?;
        for val in &self.rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// DIR

pub struct DirectoryEntryReference<'src>(pub Uri<'src>);

impl<'src> DirectoryEntryReference<'src> {
    pub const PARAM_NAME: &'static str = "DIR";
    pub fn parse_value(
        first: &'src str,
        rest: &'_ [&'src str],
    ) -> Result<Self, SingleParamError<URIError>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(DirectoryEntryReference(Uri::parse(first)?))
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

pub struct FormatType<'src>(mediatype::MediaType<'src>);

impl<'src> FormatType<'src> {
    pub const PARAM_NAME: &'static str = "FMTTYPE";
    pub fn parse_value(first: &'src str, rest: &[&'src str]) -> anyhow::Result<Self> {
        if !rest.is_empty() {
            bail!("expected single mediatype");
        }
        Ok(FormatType(MediaType::parse(first)?))
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
            other => Self::Name(Name::parse(other)?),
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

pub struct Language<'src>(LanguageTag<&'src str>);

impl<'src> Language<'src> {
    pub const PARAM_NAME: &'static str = "FMTTYPE";
    pub fn parse_value(
        first: &'src str,
        rest: &[&'src str],
    ) -> Result<Self, SingleParamError<oxilangtag::LanguageTagParseError>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(Language(LanguageTag::parse(first)?))
    }
}

impl<'src> fmt::Display for Language<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", Self::PARAM_NAME, &self.0)
    }
}

// MEMBER

pub struct GroupOrMemberList<'src> {
    pub first: CalendarUserAddress<'src>,
    pub rest: Vec<CalendarUserAddress<'src>>,
}

impl<'src> GroupOrMemberList<'src> {
    pub const PARAM_NAME: &'static str = "MEMBER";
    pub fn parse_value(first: &'src str, rest: &'_ [&'src str]) -> anyhow::Result<Self> {
        let first = CalendarUserAddress::try_from(first)?;
        let rest = rest
            .iter()
            .map(|val| CalendarUserAddress::try_from(*val))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GroupOrMemberList { first, rest })
    }
}

impl<'src> fmt::Display for GroupOrMemberList<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.first)?;
        for val in &self.rest {
            write!(f, ",\"{}\"", val)?;
        }
        Ok(())
    }
}

// PARTSTAT

// TODO these can be further subdivided if we want (for vevent, vtodo, and vjournal)
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

impl<'src> ParticipationStatus<'src> {
    pub const PARAM_NAME: &'static str = "PARTSTAT";
    pub fn parse_value(
        first: &'src str,
        rest: &[&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "NEEDS-ACTION" => Self::NeedsAction,
            "ACCEPTED" => Self::Accepted,
            "DECLINED" => Self::Declined,
            "TENTATIVE" => Self::Tentative,
            "DELEGATED" => Self::Delegated,
            "COMPLETED" => Self::Completed,
            "IN-PROCESS" => Self::InProcess,
            other => Self::Name(Name::parse(other)?),
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

pub struct Range;

impl Range {
    pub const PARAM_NAME: &'static str = "RANGE";
    pub fn parse_value(
        first: &str,
        rest: &[&str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "THISANDFUTURE" => Self,
            other => return Err(anyhow!("expected THISANDFUTURE, found `{other}`").into()),
        })
    }
}

impl Default for Range {
    fn default() -> Self {
        Self
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=THISANDFUTURE", Self::PARAM_NAME)
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
            other => Self::Name(Name::parse(other)?),
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

impl<'src> ParticipationRole<'src> {
    pub const PARAM_NAME: &'static str = "ROLE";
    pub fn parse_value(
        first: &'src str,
        rest: &[&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "CHAIR" => Self::Chair,
            "REQ-PARTICIPANT" => Self::ReqParticipant,
            "OPT-PARTICIPANT" => Self::OptParticipant,
            "NON-PARTICIPANT" => Self::NonParticipant,
            other => Self::Name(Name::parse(other)?),
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
pub enum RsvpExpectation {
    True,
    False,
}

impl RsvpExpectation {
    pub const PARAM_NAME: &'static str = "RSVP";
    pub fn parse_value(
        first: &str,
        rest: &[&str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(match first {
            "TRUE" => Self::True,
            "FALSE" => Self::False,
            other => return Err(anyhow!("expected `TRUE` or `FALSE`, found {other}").into()),
        })
    }

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

pub struct SentBy<'src>(pub CalendarUserAddress<'src>);

impl<'src> SentBy<'src> {
    pub const PARAM_NAME: &'static str = "SENT-BY";
    pub fn parse_value(
        first: &'src str,
        rest: &'_ [&'src str],
    ) -> Result<Self, SingleParamError<URIError>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        Ok(SentBy(CalendarUserAddress::try_from(first)?))
    }
}

impl<'src> fmt::Display for SentBy<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", Self::PARAM_NAME, self.0)
    }
}

// TZID

/// Timezone is not checked against database, just validated.
pub struct TimeZoneIdentifier<'src> {
    prefix: bool,
    value: &'src str,
}

impl<'src> TimeZoneIdentifier<'src> {
    pub const PARAM_NAME: &'static str = "SENT-BY";
    pub fn parse_value(
        first: &'src str,
        rest: &'_ [&'src str],
    ) -> Result<Self, SingleParamError<anyhow::Error>> {
        if !rest.is_empty() {
            return Err(SingleParamError::SingleParam);
        }
        let prefix = first.starts_with('/');
        let value = if prefix {
            // Panic: first character is ASCII so 1 byte
            param_text(&first[1..])
        } else {
            param_text(first)
        }?;
        Ok(Self { prefix, value })
    }

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
            other => Self::Name(Name::parse(other)?),
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
