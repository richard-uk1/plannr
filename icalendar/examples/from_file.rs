use std::{env::current_dir, fs};

use anyhow::Context;

pub fn main() -> anyhow::Result<()> {
    let raw = fs::read_to_string("calendar.txt").with_context(|| {
        format!(
            "cannot open {}",
            current_dir().unwrap().join("calendar.txt").display(),
        )
    })?;
    let calendar = icalendar::parse(&raw)?;
    dbg!(calendar);
    Ok(())
}
