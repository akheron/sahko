use askama::Template;
use axum::response::IntoResponse;
use chrono::{Datelike, Duration, Local, NaiveDate};
use common::prices::round_price;
use common::schedule::Schedule;

#[derive(Template)]
#[template(path = "pages/stats.html")]
struct StatsTemplate {
    stats: Vec<MonthStats>,
}

struct MonthStats {
    pub name: String,
    pub avg_price: f64,
}

impl MonthStats {
    fn for_month(year: i32, month: u32) -> Option<Self> {
        let mut date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let mut total: f64 = 0.0;
        let mut count: u32 = 0;
        loop {
            if let Some(schedule) = Schedule::load_for_date(date) {
                total += schedule.avg_price();
                count += 1;
            }
            date += Duration::days(1);
            if date.year() != year || date.month() != month {
                break;
            }
        }
        if count > 0 {
            Some(Self {
                name: format!("{:04}-{:02}", year, month),
                avg_price: round_price(total / count as f64),
            })
        } else {
            None
        }
    }
}

const START_YEAR: i32 = 2023;

pub async fn stats_route() -> impl IntoResponse {
    let today = Local::now().date_naive();
    let mut year = START_YEAR;
    let mut month = 1;
    let mut stats: Vec<MonthStats> = Vec::new();
    while year <= today.year() || (year == today.year() && month <= today.month()) {
        if let Some(month_stats) = MonthStats::for_month(year, month) {
            stats.push(month_stats);
        }
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }
    stats.reverse();
    StatsTemplate { stats }.into_response()
}
