use std::fs;

pub fn main() -> anyhow::Result<()> {
    let raw = fs::read_to_string("calendar.txt")?;
    let calendar = icalendar::parse(&raw)?;
    dbg!(calendar);
    Ok(())
}
