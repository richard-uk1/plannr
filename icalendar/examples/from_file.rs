use std::{env::current_dir, fs};

use anyhow::Context;

pub fn main() -> anyhow::Result<()> {
    let raw = fs::read_to_string("calendar.txt").with_context(|| {
        format!(
            "cannot open {}",
            current_dir().unwrap().join("calendar.txt").display(),
        )
    })?;
    let calendar = &mut icalendar::parse(&raw)?[0];

    calendar
        .events
        .sort_by(|ev1, ev2| ev1.start.unwrap().cmp(&ev2.start.unwrap()));
    dbg!(calendar);
    /*
    for event in &calendar[0].events {
        if !event.attachments.is_empty() {
            println!("{event:?}");
        }
    }
    */
    Ok(())
}
