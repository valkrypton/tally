//! Platform-independent logic for tally: activities, sampling, storage and reports.

/// Something the user can be doing when prompted, e.g. "Client A / code review".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Activity {
    pub name: String,
}

impl Activity {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_activity_from_str() {
        assert_eq!(Activity::new("Writing").name, "Writing");
    }
}
