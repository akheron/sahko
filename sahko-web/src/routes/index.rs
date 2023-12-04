use crate::date::LocalExt;
use crate::routes::schedule::ScheduleModel;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::Query;
use chrono::{Duration, Local, NaiveDate};
use common::schedule::Schedule;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct IndexQuery {
    date: Option<NaiveDate>,
}

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexTemplate {
    current_date: String,
    prev_date: Option<NaiveDate>,
    next_date: Option<NaiveDate>,
    schedule: ScheduleModel,
}

pub async fn index_route(query: Query<IndexQuery>) -> Response {
    let current_date = query
        .date
        .unwrap_or_else(|| Local::now().naive_local().date());

    let Some(schedule) = Schedule::load_for_date(current_date) else {
        return (
            StatusCode::NOT_FOUND,
            format!("Schedule not found for {}", current_date),
        )
            .into_response();
    };
    let prev_date = current_date - Duration::days(1);
    let next_date = current_date + Duration::days(1);

    IndexTemplate {
        current_date: current_date.format("%a %d.%m.%Y").to_string(),
        prev_date: Schedule::load_for_date(prev_date).map(|_| prev_date),
        next_date: Schedule::load_for_date(next_date).map(|_| next_date),
        schedule: ScheduleModel::from_pin_schedules(Local::current_hour(), current_date, &schedule),
    }
    .into_response()
}
