use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority(u8);

impl Default for Priority {
    fn default() -> Self {
        // medium
        Self(5)
    }
}

impl Priority {
    pub fn new(priority: u8) -> Self {
        Self(priority.min(9))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl FromStr for Priority {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u8 = s.parse()?;
        Ok(Self::new(v))
    }
}
