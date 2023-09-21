#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Date {
    Today,
    Tomorrow,
}

impl Date {
    pub fn format(&self, format: &str) -> String {
        match self {
            Date::Today => chrono::Local::now(),
            Date::Tomorrow => chrono::Local::now() + chrono::Duration::days(1),
        }
        .format(format)
        .to_string()
    }
}
