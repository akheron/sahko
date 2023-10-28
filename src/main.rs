mod config;
mod domain;
mod email;
mod gpio;
mod prices;
mod schedule;

use chrono::Timelike;
use std::fs::{create_dir_all, File};
use std::io::Write;

use eyre::Result;

use crate::config::{Config, ScheduleConfig};
use crate::domain::RelativeDate;
use crate::email::EmailClient;
use crate::gpio::{set_pin_states, StateChange};
use crate::prices::PriceClient;
use crate::schedule::Schedule;

const MAKE_TOMORROWS_SCHEDULE: u32 = 16; // 16:00

fn main() -> Result<()> {
    let config = Config::load("config.json")?;
    let email_client = EmailClient::new(&config.email);

    if let Err(error) = run(&config.schedules, &email_client) {
        let _ = email_client.send_error(&error);
        Err(error)
    } else {
        Ok(())
    }
}

fn run(config: &[ScheduleConfig], email_client: &EmailClient) -> Result<()> {
    let price_client = PriceClient::new();
    let now = chrono::Local::now().naive_local();

    let (schedule, created) = ensure_schedule(RelativeDate::Today, &price_client, config)?;
    if created {
        let _ = email_client.send_schedule(RelativeDate::Today, &schedule);
    }

    if now.time().hour() >= MAKE_TOMORROWS_SCHEDULE {
        match ensure_schedule(RelativeDate::Tomorrow, &price_client, config) {
            Ok((schedule, created)) => {
                if created {
                    let _ = email_client.send_schedule(RelativeDate::Tomorrow, &schedule);
                }
            }
            Err(error) => {
                let _ = email_client.send_error_making_tomorrows_schedule(&error);
            }
        }
    }

    let expected_states = schedule
        .pins
        .iter()
        .map(|pin_schedule| (pin_schedule.pin, pin_schedule.is_on(&now)))
        .collect::<Vec<_>>();

    match set_pin_states(&expected_states)? {
        StateChange::None => (),
        StateChange::Change {
            changed_pins,
            powered_on,
        } => {
            let changes = changed_pins
                .into_iter()
                .map(|i| {
                    let pin_schedule = &schedule.pins[i];
                    let (_, state) = expected_states[i];
                    (&pin_schedule.name as &str, pin_schedule.pin, state)
                })
                .collect::<Vec<_>>();
            email_client.send_pin_state_change(&changes, powered_on)?;
        }
    }

    Ok(())
}

fn ensure_schedule(
    date: RelativeDate,
    client: &PriceClient,
    config: &[ScheduleConfig],
) -> Result<(Schedule, bool)> {
    if let Some(schedule) = load_schedule_for_date(date) {
        Ok((schedule, false))
    } else {
        let prices = client.get_prices_for_date(date)?;
        let schedule = Schedule::compute(config, &prices);

        create_dir_all(SCHEDULE_DIR_NAME)?;
        write!(
            File::create(schedule_filename(date))?,
            "{}",
            serde_json::to_string_pretty(&schedule)?
        )?;
        Ok((schedule, true))
    }
}

fn load_schedule_for_date(date: RelativeDate) -> Option<Schedule> {
    let file = File::open(schedule_filename(date)).ok()?;
    serde_json::from_reader(file).ok()
}

const SCHEDULE_DIR_NAME: &str = "schedules";

fn schedule_filename(date: RelativeDate) -> String {
    format!(
        "{}/schedule_{}.json",
        SCHEDULE_DIR_NAME,
        date.format("%Y-%m-%d")
    )
}
