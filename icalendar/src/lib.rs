use std::borrow::Cow;

use crate::parser::{Lexer, print_lines};

#[macro_use]
mod macros;

mod content_line;
pub mod params;
mod parser;
pub mod types;
mod values;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

/// iCal parser
#[derive(Debug)]
pub struct Calendar<'src> {
    prod_id: Cow<'src, str>,
}

/// Parse a file in iCalendar format and return a list of calendars
pub fn parse(input: &str) -> Result<Vec<Calendar>> {
    let mut parser = Lexer::new(input);
    let mut calendars = vec![];
    while !parser.is_empty()? {
        calendars.push(Calendar::parse(&mut parser)?);
    }
    Ok(calendars)
}
