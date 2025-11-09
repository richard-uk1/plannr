//! Turns text input into lines

use std::borrow::Cow;

use anyhow::{anyhow, bail};

mod line;
use line::Line;

mod error;
pub use error::ParserError;

pub(crate) mod helpers;
mod lexer;
pub(crate) use lexer::Lexer;

mod param_map;
pub(crate) use param_map::ParamMap;

use crate::{
    AnnotatedText, Attachment, Attendee, CalScale, Calendar, Categories, Class, Comment, Contact,
    Event, EventEnd, EventStatus, ExceptionDateTimes, Organizer, RecurrenceId, Result,
    TimeTransparency,
    params::{
        CommonName, Delegatees, Delegators, DirectoryEntryReference, GroupOrListMember, Language,
        SentBy,
    },
    parser::helpers::{
        check_iana_token, opt_vec_one_to_vec, parse_date_or_datetime, parse_date_or_datetime_list,
    },
    types::{Data, DateOrDateTime, DateTime, Duration, GeoLocation, Name, Priority},
    values::Text,
};

const ENCODING_PARAM: Name = Name::iana("ENCODING");
const VALUE_PARAM: Name = Name::iana("VALUE");

/// Macor for builders that expect 0 or 1 instances of a field
macro_rules! impl_set_01 {
    ($id:ident, $setter:ident, $ty:ty, $label:literal) => {
        fn $setter(&mut self, $id: $ty) -> Result {
            if self.$id.is_some() {
                bail!(concat!("expected 0..=1 ", $label, ", found at least 2"));
            }
            self.$id = Some($id);
            Ok(())
        }
    };
}

macro_rules! impl_set_1 {
    ($id:ident, $setter:ident, $ty:ty, $label:literal) => {
        fn $setter(&mut self, $id: $ty) -> Result {
            if self.$id.is_some() {
                bail!(concat!("expected 1 ", $label, ", found at least 2"));
            }
            self.$id = Some($id);
            Ok(())
        }
    };
}

impl<'src> Calendar<'src> {
    // Only this parse fn handles the BEGIN line, all others assume this was already parsed.
    pub(crate) fn parse(parser: &mut Lexer<'src>) -> Result<Self> {
        let Some(begin) = parser.take_next()? else {
            bail!("empty iterator: call is_empty before this function to avoid");
        };
        if !(&begin.name == "BEGIN" && begin.value == "VCALENDAR") {
            bail!("expected `BEGIN:VCALENDAR`");
        }

        let mut builder = CalendarBuilder::new();
        while let Some(next) = parser.take_next()? {
            if &next.name == "END" {
                if next.value != "VCALENDAR" {
                    bail!("expected VCALENDAR, found {}", next.value);
                }
                return Ok(builder.build()?);
            } else if &next.name == "PRODID" {
                builder.set_prod_id(parse_prodid(next)?)?;
            } else if &next.name == "VERSION" {
                builder.set_version(parse_version(next)?)?;
            } else if &next.name == "CALSCALE" {
                builder.set_cal_scale(parse_cal_scale(next)?)?;
            } else if &next.name == "METHOD" {
                check_iana_token(&next.value)?;
                builder.set_method(next.value)?;
            } else if &next.name == "BEGIN" {
                // VEVENT, VTODO, etc.
                if next.value == "VEVENT" {
                    builder.events.push(Event::parse(parser)?);
                } else {
                    // TODO error instead?
                    parser.skip_current()?;
                }
            }
        }
        bail!("unexpected EOF");
    }
}

impl<'src> Event<'src> {
    fn parse(parser: &mut Lexer<'src>) -> Result<Self> {
        let mut builder = EventBuilder::default();
        while let Some(next) = parser.take_next()? {
            if &next.name == "END" {
                if next.value != "VEVENT" {
                    bail!("expected VEVENT, found {}", next.value);
                }
                return Ok(builder.build()?);
            } else if &next.name == "CLASS" {
                builder.set_class(parse_class(next.value)?)?;
            } else if &next.name == "CREATED" {
                builder.set_created(DateTime::parse(&*next.value)?.1)?;
            } else if &next.name == "DESCRIPTION" {
                builder.set_description(parse_annotated_text(next)?)?;
            } else if &next.name == "DTSTART" {
                builder.set_start(DateOrDateTime::parse(&*next.value)?.1)?;
            } else if &next.name == "GEO" {
                builder.set_geo_location(next.value.parse()?)?;
            } else if &next.name == "LAST-MODIFIED" {
                builder.set_last_modified(DateTime::parse(&*next.value)?.1)?;
            } else if &next.name == "LOCATION" {
                builder.set_location(parse_annotated_text(next)?)?;
            } else if &next.name == "ORGANIZER" {
                builder.set_organizer(parse_organizer(next)?)?;
            } else if &next.name == "PRIORITY" {
                builder.set_priority(next.value.parse()?)?;
            } else if &next.name == "DTSTAMP" {
                builder.set_timestamp(DateTime::parse(&*next.value)?.1)?;
            } else if &next.name == "SEQ" {
                builder.set_sequence(next.value.parse()?)?;
            } else if &next.name == "STATUS" {
                builder.set_status(parse_event_status(next)?)?;
            } else if &next.name == "SUMMARY" {
                builder.set_summary(parse_annotated_text(next)?)?;
            } else if &next.name == "TRANSP" {
                builder.set_time_transparency(parse_time_transparency(next)?)?;
            } else if &next.name == "UID" {
                builder.set_uid(next.value)?;
            } else if &next.name == "RECURRENCE-ID" {
                builder.set_recurrence_id(parse_recurrence_id(next)?)?;
            } else if &next.name == "DTEND" {
                builder.set_end(parse_datetime_end(next)?)?;
            } else if &next.name == "DURATION" {
                builder.set_end(EventEnd::Duration(Duration::parse(&*next.value)?.1))?;
            } else if &next.name == "ATTACHMENT" {
                builder.attachments.push(parse_attachment(next)?);
            } else if &next.name == "ATTENDEE" {
                builder.attendees.push(parse_attendee(next)?);
            } else if &next.name == "CATEGORIES" {
                builder.categories.push(parse_categories(next)?);
            } else if &next.name == "COMMENT" {
                builder.comments.push(parse_comment(next)?);
            } else if &next.name == "CONTACT" {
                builder.contacts.push(parse_contact(next)?);
            } else if &next.name == "EXDATE" {
                builder.exception_dates.push(parse_exception_dates(next)?);
            } else if &next.name == "BEGIN" {
                // skip all other subtrees
                parser.skip_current()?;
            }
        }
        bail!("unexpected EOF")
    }
}

fn parse_prodid<'src>(input: Line<'src>) -> Result<Cow<'src, str>> {
    debug_assert_eq!(&input.name, "PRODID");
    if let Some(param) = input.first_iana_param() {
        bail!("unexpected param {param:?}");
    }
    Ok(input.value)
}

fn parse_version<'src>(input: Line<'src>) -> Result {
    debug_assert_eq!(&input.name, "VERSION");
    if let Some(param) = input.first_iana_param() {
        bail!("unexpected param {param:?}");
    }
    if input.value != "2.0" {
        bail!("only version 2.0 supported");
    }
    Ok(())
}

fn parse_cal_scale<'src>(input: Line<'src>) -> Result<CalScale<'src>> {
    debug_assert_eq!(&input.name, "CALSCALE");
    if let Some(param) = input.first_iana_param() {
        bail!("unexpected param {param:?}");
    }
    if input.value == "GREGORIAN" {
        Ok(CalScale::Gregorian)
    } else {
        check_iana_token(&input.value)?;
        Ok(CalScale::Other(input.value))
    }
}

fn parse_class<'src>(input: Cow<'src, str>) -> Result<Class<'src>> {
    if input == "PUBLIC" {
        Ok(Class::Public)
    } else if input == "PRIVATE" {
        Ok(Class::Private)
    } else if input == "CONFIDENTIAL" {
        Ok(Class::Confidential)
    } else {
        match Name::parse(input)? {
            Name::XName(xname) => Ok(Class::XName(xname)),
            Name::Iana(cow) => Ok(Class::Iana(cow)),
        }
    }
}

fn parse_annotated_text<'src>(mut input: Line<'src>) -> Result<AnnotatedText<'src>> {
    let lang = input.params.take_ty()?;
    let altrep = input.params.take_ty()?;

    Ok(AnnotatedText {
        lang,
        altrep,
        text: input.value,
    })
}

fn parse_organizer<'src>(mut input: Line<'src>) -> Result<Organizer<'src>> {
    let common_name = input.params.take_ty::<CommonName<'src>>()?;
    let dir = input.params.take_ty()?;
    let sent_by = input.params.take_ty()?;
    let lang = input.params.take_ty()?;
    let value = input.value.try_into()?;

    Ok(Organizer {
        dir,
        common_name: common_name.map(|v| v.0),
        sent_by,
        lang,
        value,
    })
}

fn parse_event_status(input: Line<'_>) -> Result<EventStatus> {
    match &*input.value {
        "TENTATIVE" => Ok(EventStatus::Tentative),
        "CONFIRMED" => Ok(EventStatus::Confirmed),
        "CANCELLED" => Ok(EventStatus::Cancelled),
        other => bail!("unexpected status {other}"),
    }
}

fn parse_time_transparency(input: Line<'_>) -> Result<TimeTransparency> {
    match &*input.value {
        "OPAQUE" => Ok(TimeTransparency::Opaque),
        "TRANSPARENT" => Ok(TimeTransparency::Transparent),
        other => bail!("unexpected time transparency value {other}"),
    }
}

fn parse_recurrence_id<'src>(mut input: Line<'src>) -> Result<RecurrenceId<'src>> {
    let range = input.params.take_ty()?;
    let timezone_id = input.params.take_ty()?;
    let value = parse_date_or_datetime(&mut input)?;

    Ok(RecurrenceId {
        range,
        timezone_id,
        value,
    })
}

fn parse_datetime_end<'src>(mut input: Line<'src>) -> Result<EventEnd<'src>> {
    let timezone_id = input.params.take_ty()?;

    let value = parse_date_or_datetime(&mut input)?;

    Ok(EventEnd::DateTime { value, timezone_id })
}

fn parse_attachment<'src>(mut input: Line<'src>) -> Result<Attachment<'src>> {
    let fmt_type = input.params.take_ty()?;
    let data = if let Some(v) = input.params.take(&VALUE_PARAM) {
        let v = v.get_single()?;
        if v != "BINARY" {
            bail!("only BINARY value is allowed");
        }
        let Some(enc) = input.params.take(&ENCODING_PARAM) else {
            bail!("cannot have VALUE without ENCODING");
        };
        let enc = enc.get_single()?;
        if enc != "BASE64" {
            bail!("only BASE64 encoding is allowed");
        }
        Data::parse_blob(input.value)?
    } else {
        Data::parse_uri(input.value)?
    };

    Ok(Attachment { fmt_type, data })
}

fn parse_attendee<'src>(mut input: Line<'src>) -> Result<Attendee<'src>> {
    let cutype = input.params.take_ty()?;
    let group_or_list_members = input.params.take_ty::<GroupOrListMember<'src>>()?;
    let group_or_list_members = opt_vec_one_to_vec(group_or_list_members.map(|list| list.0));

    let role = input.params.take_ty()?;
    let participation_status = input.params.take_ty()?;
    let rsvp = input.params.take_ty()?;

    let delegated_to = input.params.take_ty::<Delegatees<'src>>()?;
    let delegated_to = opt_vec_one_to_vec(delegated_to.map(|list| list.0));

    let delegated_from = input.params.take_ty::<Delegators<'src>>()?;
    let delegated_from = opt_vec_one_to_vec(delegated_from.map(|list| list.0));

    let sent_by = input.params.take_ty::<SentBy>()?;
    let cn = input.params.take_ty::<CommonName<'src>>()?;
    let dir = input.params.take_ty::<DirectoryEntryReference<'src>>()?;
    let lang = input.params.take_ty::<Language<'src>>()?;

    Ok(Attendee {
        cutype: cutype.unwrap_or_default(),
        group_or_list_members,
        role: role.unwrap_or_default(),
        participation_status: participation_status.unwrap_or_default(),
        rsvp: rsvp.unwrap_or_default(),
        delegated_to,
        delegated_from,
        sent_by: sent_by.map(|v| v.0),
        common_name: cn.map(|v| v.0),
        dir,
        lang,
    })
}

fn parse_categories<'src>(mut input: Line<'src>) -> Result<Categories<'src>> {
    let lang = input.params.take_ty()?;
    let values = Text::try_from(input.value)?;

    Ok(Categories {
        lang,
        values: values.0,
    })
}

fn parse_comment<'src>(mut input: Line<'src>) -> Result<Comment<'src>> {
    let lang = input.params.take_ty()?;
    let altrep = input.params.take_ty()?;

    Ok(Comment {
        lang,
        altrep,
        value: input.value,
    })
}

fn parse_contact<'src>(mut input: Line<'src>) -> Result<Contact<'src>> {
    let lang = input.params.take_ty()?;
    let altrep = input.params.take_ty()?;

    Ok(Contact {
        lang,
        altrep,
        value: input.value,
    })
}

fn parse_exception_dates<'src>(mut input: Line<'src>) -> Result<ExceptionDateTimes<'src>> {
    let timezone_id = input.params.take_ty()?;
    let values = parse_date_or_datetime_list(&mut input)?;
    Ok(ExceptionDateTimes {
        timezone_id,
        values,
    })
}

struct CalendarBuilder<'src> {
    prod_id: Option<Cow<'src, str>>,
    version_set: bool,
    cal_scale: Option<CalScale<'src>>,
    method: Option<Cow<'src, str>>,
    events: Vec<Event<'src>>,
}

impl<'src> CalendarBuilder<'src> {
    fn new() -> Self {
        Self {
            prod_id: None,
            version_set: false,
            cal_scale: None,
            method: None,
            events: vec![],
        }
    }

    fn build(self) -> Result<Calendar<'src>> {
        Ok(Calendar {
            prod_id: self
                .prod_id
                .ok_or_else(|| anyhow!("PRODID not specified"))?,
            cal_scale: self.cal_scale.unwrap_or_default(),
            method: self.method,
            events: self.events,
        })
    }

    impl_set_01!(prod_id, set_prod_id, Cow<'src, str>, "PRODID");

    fn set_version(&mut self, (): ()) -> Result {
        if self.version_set {
            bail!("expected 1 VERSION, found at least 2");
        }
        Ok(())
    }

    impl_set_01!(cal_scale, set_cal_scale, CalScale<'src>, "CALSCALE");
    impl_set_01!(method, set_method, Cow<'src, str>, "METHOD");
}

#[derive(Default)]
pub struct EventBuilder<'src> {
    class: Option<Class<'src>>,
    created: Option<DateTime>,
    description: Option<AnnotatedText<'src>>,
    start: Option<DateOrDateTime>,
    geo: Option<GeoLocation>,
    last_modified: Option<DateTime>,
    location: Option<AnnotatedText<'src>>,
    organizer: Option<Organizer<'src>>,
    priority: Option<Priority>,
    timestamp: Option<DateTime>,
    sequence: Option<u64>,
    status: Option<EventStatus>,
    summary: Option<AnnotatedText<'src>>,
    time_transparency: Option<TimeTransparency>,
    uid: Option<Cow<'src, str>>,
    recurrence_id: Option<RecurrenceId<'src>>,
    end: Option<EventEnd<'src>>,
    attachments: Vec<Attachment<'src>>,
    attendees: Vec<Attendee<'src>>,
    categories: Vec<Categories<'src>>,
    comments: Vec<Comment<'src>>,
    contacts: Vec<Contact<'src>>,
    exception_dates: Vec<ExceptionDateTimes<'src>>,
}

impl<'src> EventBuilder<'src> {
    impl_set_01!(class, set_class, Class<'src>, "CLASS");

    fn set_created(&mut self, created: DateTime) -> Result {
        if self.created.is_some() {
            bail!("expected 0..=1 CREATED, found at least 2");
        }
        if !created.time.utc {
            bail!("expected UTC time");
        }
        self.created = Some(created);
        Ok(())
    }

    impl_set_01!(
        description,
        set_description,
        AnnotatedText<'src>,
        "DESCRIPTION"
    );
    impl_set_01!(start, set_start, DateOrDateTime, "START");
    impl_set_01!(geo, set_geo_location, GeoLocation, "GEO");
    impl_set_01!(last_modified, set_last_modified, DateTime, "LAST-MODIFIED");
    impl_set_01!(location, set_location, AnnotatedText<'src>, "LOCATION");
    impl_set_01!(organizer, set_organizer, Organizer<'src>, "ORGANIZER");
    impl_set_01!(priority, set_priority, Priority, "PRIORITY");
    impl_set_01!(timestamp, set_timestamp, DateTime, "DTSTAMP");
    impl_set_01!(sequence, set_sequence, u64, "SEQ");
    impl_set_01!(status, set_status, EventStatus, "STATUS");
    impl_set_01!(summary, set_summary, AnnotatedText<'src>, "SUMMARY");
    impl_set_01!(
        time_transparency,
        set_time_transparency,
        TimeTransparency,
        "TRANSP"
    );
    impl_set_1!(uid, set_uid, Cow<'src, str>, "UID");
    impl_set_01!(
        recurrence_id,
        set_recurrence_id,
        RecurrenceId<'src>,
        "RECURRENCE-ID"
    );

    fn set_end(&mut self, end: EventEnd<'src>) -> Result {
        if self.end.is_some() {
            bail!("expected 0..1 of DTEND | DURATION, found at least 2");
        }

        self.end = Some(end);
        Ok(())
    }

    fn build(self) -> Result<Event<'src>> {
        let Some(uid) = self.uid else {
            bail!("missing UID on VEVENT");
        };
        Ok(Event {
            class: self.class.unwrap_or_default(),
            created: self.created,
            description: self.description,
            start: self.start,
            geo_location: self.geo,
            last_modified: self.last_modified,
            location: self.location,
            organizer: self.organizer,
            priority: self.priority,
            timestamp: self.timestamp,
            sequence: self.sequence,
            status: self.status,
            summary: self.summary,
            time_transparency: self.time_transparency.unwrap_or_default(),
            uid,
            recurrence_id: self.recurrence_id,
            end: self.end,
            attachments: self.attachments,
            attendees: self.attendees,
            categories: self.categories,
            comments: self.comments,
            contacts: self.contacts,
            exception_dates: self.exception_dates,
        })
    }
}
