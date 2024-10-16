mod gpio;

use chrono::NaiveTime;
use eyre::{eyre, Result};
use pico_args::Arguments;
use std::path::PathBuf;
use std::time::Duration;

use common::config::{Config, ScheduleConfig};
use common::domain::RelativeDate;
use common::email::EmailClient;
use common::prices::PriceClient;
use common::schedule::Schedule;

use crate::gpio::{set_pin_states, StateChange};

const MAKE_TOMORROWS_SCHEDULE: (u32, u32) = (15, 5);

fn main() -> Result<()> {
    let mut args = Arguments::from_env();

    let config = Config::load("config.json")?;
    let email_client = EmailClient::new(&config.email);

    if args.contains(["-h", "--help"]) {
        let bin = PathBuf::from(std::env::args_os().next().unwrap_or_default());
        let bin = bin.file_name().unwrap_or_default().to_string_lossy();
        println!("Usage: {bin} [--send-schedules]");
        return Ok(());
    }
    if args.contains("--send-schedules") {
        send_schedules(&config, &email_client)
    } else if let Err(error) = run(&config.schedules, &email_client) {
        let _ = email_client.send_error(&error);
        Err(error)
    } else {
        Ok(())
    }
}

fn send_schedules(config: &Config, email_client: &EmailClient) -> Result<()> {
    let price_client = PriceClient::new();
    for date in [RelativeDate::Today, RelativeDate::Tomorrow] {
        let (schedule, _) = ensure_schedule(date, &price_client, &config.schedules)?;
        let _ = email_client.send_schedule(date.to_naive_date(), &schedule);
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

fn run(config: &[ScheduleConfig], email_client: &EmailClient) -> Result<()> {
    let price_client = PriceClient::new();
    let now = chrono::Local::now();

    let (schedule, created) = ensure_schedule(RelativeDate::Today, &price_client, config)?;
    if created {
        let _ = email_client.send_schedule(RelativeDate::Today.to_naive_date(), &schedule);
    }

    let tomorrow_available =
        NaiveTime::from_hms_opt(MAKE_TOMORROWS_SCHEDULE.0, MAKE_TOMORROWS_SCHEDULE.1, 0).unwrap();
    if now.time() >= tomorrow_available {
        match ensure_schedule(RelativeDate::Tomorrow, &price_client, config) {
            Ok((schedule, created)) => {
                if created {
                    let _ = email_client
                        .send_schedule(RelativeDate::Tomorrow.to_naive_date(), &schedule);
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
                    (&pin_schedule.name as &str, state)
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
    if let Some(schedule) = Schedule::load_for_date(date.to_naive_date()) {
        Ok((schedule, false))
    } else {
        let prices = client.get_prices_for_date(date)?;
        // DST change day may only have 23 entries
        if prices.len() < 23 {
            return Err(eyre!("Incomplete prices for {}", date.to_naive_date()));
        }
        let schedule = Schedule::compute(config, &prices);
        schedule.write_to_file(date.to_naive_date())?;
        Ok((schedule, true))
    }
}
