use crate::parser::{line_iter, print_lines};

mod line_iter;
mod parser;

/// iCal parser
pub fn parse(input: &str) -> anyhow::Result<Calendar> {
    print_lines(input);
    todo!()
}

#[derive(Debug)]
pub struct Calendar {}
