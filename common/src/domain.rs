use chrono::{DateTime, Days, Duration, Local, NaiveDate, Timelike, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeDate {
    Today,
    Tomorrow,
}

impl RelativeDate {
    pub fn to_start_and_end(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = Local::now()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            + Days::new(match self {
                RelativeDate::Today => 0,
                RelativeDate::Tomorrow => 1,
            });
        let end = start + Days::new(1) - Duration::seconds(1);
        (start.with_timezone(&Utc), end.with_timezone(&Utc))
    }

    pub fn to_naive_date(&self) -> NaiveDate {
        match self {
            RelativeDate::Today => Local::now(),
            RelativeDate::Tomorrow => Local::now() + Days::new(1),
        }
        .naive_local()
        .date()
    }
}
