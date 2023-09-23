#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeDate {
    Today,
    Tomorrow,
}

impl RelativeDate {
    pub fn format(&self, format: &str) -> String {
        match self {
            RelativeDate::Today => chrono::Local::now(),
            RelativeDate::Tomorrow => chrono::Local::now() + chrono::Duration::days(1),
        }
        .format(format)
        .to_string()
    }
}
