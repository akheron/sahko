use chrono::Timelike;
use std::fs::{create_dir_all, File};
use std::io::Write;

use eyre::Result;

use sahko::config::{Config, ScheduleConfig};
use sahko::domain::Date;
use sahko::email::EmailClient;
use sahko::gpio::OutputPin;
use sahko::prices::PriceClient;
use sahko::schedule::{PinSchedule, Schedule};

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

    let (schedule, created) = ensure_schedule(Date::Today, &price_client, config)?;
    if created {
        let _ = email_client.send_schedule(Date::Today, &schedule);
    }

    if now.time().hour() >= MAKE_TOMORROWS_SCHEDULE {
        let (schedule, created) = ensure_schedule(Date::Tomorrow, &price_client, config)?;
        if created {
            let _ = email_client.send_schedule(Date::Tomorrow, &schedule);
        }
    }

    for schedule in schedule {
        let mut pin = OutputPin::new(schedule.pin)?;

        let current_state = pin.state();
        let expected_state = schedule.is_on(&now);

        if current_state != expected_state {
            pin.set(expected_state);
            let _ =
                email_client.send_pin_state_change(&schedule.name, schedule.pin, expected_state);
        }
    }

    Ok(())
}

fn ensure_schedule(
    date: Date,
    client: &PriceClient,
    config: &[ScheduleConfig],
) -> Result<(Schedule, bool)> {
    if let Some(schedule) = load_schedule_for_date(date) {
        Ok((schedule, false))
    } else {
        let prices = client.get_prices_for_date(date)?;
        let schedule = config
            .iter()
            .map(|schedule| PinSchedule::compute(schedule, &prices))
            .collect::<Vec<_>>();

        create_dir_all(SCHEDULE_DIR_NAME)?;
        write!(
            File::create(schedule_filename(date))?,
            "{}",
            serde_json::to_string_pretty(&schedule)?
        )?;
        Ok((schedule, true))
    }
}

fn load_schedule_for_date(date: Date) -> Option<Vec<PinSchedule>> {
    let file = File::open(schedule_filename(date)).ok()?;
    serde_json::from_reader(file).ok()
}

const SCHEDULE_DIR_NAME: &str = "schedules";

fn schedule_filename(date: Date) -> String {
    format!(
        "{}/schedule_{}.json",
        SCHEDULE_DIR_NAME,
        date.format("%Y-%m-%d")
    )
}
