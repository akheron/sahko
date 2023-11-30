use chrono::{Days, Local, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeDate {
    Today,
    Tomorrow,
}

impl RelativeDate {
    pub fn to_naive_date(&self) -> NaiveDate {
        match self {
            RelativeDate::Today => Local::now(),
            RelativeDate::Tomorrow => Local::now() + Days::new(1),
        }
        .naive_local()
        .date()
    }
}
