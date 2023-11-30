use askama::Template;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_extra::extract::Form;
use chrono::NaiveDate;
use common::schedule::Schedule;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

use crate::date::NaiveDateExt;
use crate::lock::WriteLock;
use crate::response::HtmlTemplate;

pub async fn update_schedule_route(
    Extension(write_lock): Extension<WriteLock>,
    Form(body): Form<UpdateScheduleBody>,
) -> Response {
    let Some(mut schedule) = Schedule::load_for_date(body.date) else {
        return (
            StatusCode::NOT_FOUND,
            format!("Schedule not found for {}", body.date),
        )
            .into_response();
    };
    let is_on = |pin: u8, hour_index: u32| {
        body.pin_hours
            .iter()
            .any(|(p, h)| *p == pin && *h == hour_index)
    };

    for pin in &mut schedule.pins {
        pin.on_hours = body
            .date
            .iter_hours()
            .enumerate()
            .filter_map(|(hour_index, hour)| {
                if is_on(pin.pin, hour_index as u32) {
                    Some(hour.fixed_offset())
                } else {
                    None
                }
            })
            .collect();
    }

    {
        // Guard against concurrent writes
        let _unused = write_lock.lock();
        schedule.write_to_file(body.date).unwrap();
    }

    HtmlTemplate(ScheduleTemplate {
        schedule: ScheduleModel::from_pin_schedules(body.date, &schedule),
    })
    .into_response()
}

#[derive(Deserialize)]
pub struct UpdateScheduleBody {
    date: NaiveDate,

    #[serde(deserialize_with = "deserialize_pairs")]
    pin_hours: Vec<(u8, u32)>,
}

fn deserialize_pairs<'de, D, T1, T2>(deserializer: D) -> Result<Vec<(T1, T2)>, D::Error>
where
    D: Deserializer<'de>,
    T1: FromStr,
    T2: FromStr,
{
    let hours: Vec<String> = Vec::deserialize(deserializer)?;
    let err = || Error::custom("Invalid format");
    hours
        .iter()
        .map(|hour| {
            let (pin, hour_index) = hour.split_once(',').ok_or_else(err)?;
            let pin = pin.parse::<T1>().map_err(|_| err())?;
            let hour_index = hour_index.parse::<T2>().map_err(|_| err())?;
            Ok((pin, hour_index))
        })
        .collect::<Result<Vec<_>, _>>()
}

#[derive(Template)]
#[template(path = "components/schedule.html")]
struct ScheduleTemplate {
    schedule: ScheduleModel,
}

pub struct ScheduleModel {
    pub date: NaiveDate,
    pub pins: Vec<PinInfo>,
    pub avg_price: f64,
}

pub struct PinInfo {
    pub name: String,
    pub pin: u8,
    pub hours: Vec<HourInfo>,
    pub avg_price: f64,
}

pub struct HourInfo {
    pub hour: String,
    pub on: bool,
    pub price: f64,
}

impl ScheduleModel {
    pub fn from_pin_schedules(date: NaiveDate, schedule: &Schedule) -> Self {
        Self {
            date,
            pins: schedule
                .pins
                .iter()
                .map(|pin| PinInfo {
                    name: pin.name.clone(),
                    pin: pin.pin,
                    hours: date
                        .iter_hours()
                        .map(|hour| {
                            let on = pin.on_hours.iter().any(|&t| t == hour);
                            HourInfo {
                                hour: hour.format("%H").to_string(),
                                on,
                                price: schedule.price_for_hour(hour).unwrap(),
                            }
                        })
                        .collect(),
                    avg_price: pin.avg_price(&schedule.prices, true),
                })
                .collect(),
            avg_price: schedule.avg_price(),
        }
    }
}
