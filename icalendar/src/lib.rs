//!
//! A crate for parsing (and possibly in the future) serializing calendar data to
//! CalDAV iCalendar format.
//!
//!
use std::borrow::Cow;

use crate::{
    params::{
        AlternativeTextRepresentation, CalendarUserType, DirectoryEntryReference, FormatType,
        Language, ParticipationRole, ParticipationStatus, Range, RsvpExpectation, SentBy,
        TimeZoneIdentifier,
    },
    parser::Lexer,
    types::{Data, DateOrDateTime, DateTime, Duration, GeoLocation, Priority, VecOne, XName},
    values::CalendarUserAddress,
};

#[macro_use]
mod macros;

pub mod params;
pub(crate) mod parser;
pub mod types;
mod values;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

/// Parse a file in iCalendar format and return a list of calendars
pub fn parse(input: &str) -> Result<Vec<Calendar>> {
    let mut parser = Lexer::new(input);
    let mut calendars = vec![];
    while !parser.is_empty()? {
        calendars.push(Calendar::parse(&mut parser)?);
    }
    Ok(calendars)
}

/// iCal parser
#[derive(Debug)]
pub struct Calendar<'src> {
    pub events: Vec<Event<'src>>,
    pub prod_id: Cow<'src, str>,
    pub cal_scale: CalScale<'src>,
    pub method: Option<Cow<'src, str>>,
}

#[derive(Debug)]
pub struct Event<'src> {
    pub class: Class<'src>,
    pub created: Option<DateTime>,
    pub last_modified: Option<DateTime>,
    pub description: Option<AnnotatedText<'src>>,
    pub start: Option<DateOrDateTime>,
    pub location: Option<AnnotatedText<'src>>,
    pub geo_location: Option<GeoLocation>,
    pub organizer: Option<Organizer<'src>>,
    pub priority: Option<Priority>,
    pub timestamp: Option<DateTime>,
    pub sequence: Option<u64>,
    pub status: Option<EventStatus>,
    pub summary: Option<AnnotatedText<'src>>,
    pub time_transparency: TimeTransparency,
    pub uid: Cow<'src, str>,
    pub recurrence_id: Option<RecurrenceId<'src>>,
    pub end: Option<EventEnd<'src>>,
    pub attachments: Vec<Attachment<'src>>,
    pub attendees: Vec<Attendee<'src>>,
    pub categories: Vec<Categories<'src>>,
    pub comments: Vec<Comment<'src>>,
    pub contacts: Vec<Contact<'src>>,
    pub exception_dates: Vec<ExceptionDateTimes<'src>>,
}

#[derive(Debug, Default)]
pub enum CalScale<'src> {
    #[default]
    Gregorian,
    Other(Cow<'src, str>),
}

#[derive(Debug, Default)]
pub enum Class<'src> {
    #[default]
    Public,
    Private,
    Confidential,
    Iana(Cow<'src, str>),
    XName(XName<'src>),
}

/// Text that has optional language and alt representation
#[derive(Default, Debug)]
pub struct AnnotatedText<'src> {
    pub lang: Option<Language<'src>>,
    pub altrep: Option<AlternativeTextRepresentation<'src>>,
    pub text: Cow<'src, str>,
}

#[derive(Debug)]
pub struct Organizer<'src> {
    pub common_name: Option<Cow<'src, str>>,
    pub dir: Option<DirectoryEntryReference<'src>>,
    pub sent_by: Option<SentBy<'src>>,
    pub lang: Option<Language<'src>>,
    pub value: CalendarUserAddress<'src>,
}

#[derive(Debug)]
pub enum EventStatus {
    Tentative,
    Confirmed,
    Cancelled,
}

#[derive(Debug, Default)]
pub enum TimeTransparency {
    #[default]
    Opaque,
    Transparent,
}

#[derive(Debug)]
pub struct RecurrenceId<'src> {
    pub range: Option<Range>,
    pub timezone_id: Option<TimeZoneIdentifier<'src>>,
    pub value: DateOrDateTime,
}

#[derive(Debug)]
pub enum EventEnd<'src> {
    DateTime {
        value: DateOrDateTime,
        timezone_id: Option<TimeZoneIdentifier<'src>>,
    },
    Duration(Duration),
}

#[derive(Debug)]
pub struct Attachment<'src> {
    pub fmt_type: Option<FormatType<'src>>,
    pub data: Data<'src>,
}

#[derive(Debug)]
pub struct Attendee<'src> {
    pub cutype: CalendarUserType<'src>,
    pub group_or_list_members: Vec<CalendarUserAddress<'src>>,
    pub role: ParticipationRole<'src>,
    pub participation_status: ParticipationStatus<'src>,
    pub rsvp: RsvpExpectation,
    pub delegated_to: Vec<CalendarUserAddress<'src>>,
    pub delegated_from: Vec<CalendarUserAddress<'src>>,
    pub sent_by: Option<CalendarUserAddress<'src>>,
    pub common_name: Option<Cow<'src, str>>,
    pub dir: Option<DirectoryEntryReference<'src>>,
    pub lang: Option<Language<'src>>,
}

#[derive(Debug)]
pub struct Categories<'src> {
    pub lang: Option<Language<'src>>,
    pub values: VecOne<Cow<'src, str>>,
}

#[derive(Debug)]
pub struct Comment<'src> {
    pub lang: Option<Language<'src>>,
    pub altrep: Option<AlternativeTextRepresentation<'src>>,
    pub value: Cow<'src, str>,
}

#[derive(Debug)]
pub struct Contact<'src> {
    pub lang: Option<Language<'src>>,
    pub altrep: Option<AlternativeTextRepresentation<'src>>,
    pub value: Cow<'src, str>,
}

#[derive(Debug)]
pub struct ExceptionDateTimes<'src> {
    pub timezone_id: Option<TimeZoneIdentifier<'src>>,
    pub values: VecOne<DateOrDateTime>,
}
