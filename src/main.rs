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
use crate::gpio::OutputPin;
use crate::prices::PriceClient;
use crate::schedule::Schedule;

const MAKE_TOMORROWS_SCHEDULE: u32 = 17; // 17:00

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
        let (schedule, created) = ensure_schedule(RelativeDate::Tomorrow, &price_client, config)?;
        if created {
            let _ = email_client.send_schedule(RelativeDate::Tomorrow, &schedule);
        }
    }

    for pin_schedule in schedule.pins {
        let mut pin = OutputPin::new(pin_schedule.pin)?;

        let current_state = pin.state();
        let expected_state = pin_schedule.is_on(&now);

        if current_state != expected_state {
            pin.set(expected_state);
            let _ = email_client.send_pin_state_change(
                &pin_schedule.name,
                pin_schedule.pin,
                expected_state,
            );
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
