use crate::parser::print_lines;

mod content_line;
mod line_iter;
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
