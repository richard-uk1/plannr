use crate::parser::print_lines;

#[macro_use]
mod macros;

mod content_line;
pub mod params;
mod parser;
pub mod types;
mod values;

/// iCal parser
pub fn parse(input: &str) -> anyhow::Result<Calendar> {
    print_lines(input);
    todo!()
}

#[derive(Debug)]
pub struct Calendar {}
