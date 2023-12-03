use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum_extra::extract::Form;
use chrono::NaiveDate;
use common::config::Config;
use common::email::EmailClient;
use common::schedule::Schedule;
use serde::Deserialize;

pub async fn send_email_route(Form(body): Form<SendEmailBody>) -> impl IntoResponse {
    let Ok(config) = Config::load("config.json") else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    let email_client = EmailClient::new(&config.email);
    let Some(schedule) = Schedule::load_for_date(body.date) else {
        return StatusCode::NOT_FOUND;
    };
    let Ok(_) = email_client.send_schedule(body.date, &schedule) else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
pub struct SendEmailBody {
    date: NaiveDate,
}
