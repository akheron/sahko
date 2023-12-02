use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, Timelike};

pub trait NaiveDateExt {
    fn start_of_day(&self) -> DateTime<Local>;
    fn iter_hours(&self) -> HourIterator;
}

impl NaiveDateExt for NaiveDate {
    fn start_of_day(&self) -> DateTime<Local> {
        self.and_time(NaiveTime::MIN)
            .and_local_timezone(Local)
            .unwrap()
    }

    fn iter_hours(&self) -> HourIterator {
        HourIterator(Some(self.start_of_day()))
    }
}

pub struct HourIterator(Option<DateTime<Local>>);

impl Iterator for HourIterator {
    type Item = DateTime<Local>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0;
        if let Some(cur) = self.0 {
            let next = cur + Duration::hours(1);
            if next.date_naive() == cur.date_naive() {
                self.0 = Some(next);
            } else {
                self.0 = None
            }
        }
        result
    }
}

pub trait LocalExt {
    fn current_hour() -> DateTime<Local>;
}

impl LocalExt for Local {
    fn current_hour() -> DateTime<Local> {
        Self::now()
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap()
    }
}
