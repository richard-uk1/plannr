use core::fmt;
use std::str::FromStr;

use anyhow::bail;

#[derive(Debug)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
}

impl FromStr for GeoLocation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((latitude, longitude)) = s.split_once(';') else {
            bail!("expected ';'");
        };
        Ok(Self {
            latitude: latitude.parse()?,
            longitude: longitude.parse()?,
        })
    }
}

impl fmt::Display for GeoLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.latitude, f)?;
        f.write_str(";")?;
        fmt::Display::fmt(&self.longitude, f)
    }
}
