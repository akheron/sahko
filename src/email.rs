use chrono::{DateTime, Duration, FixedOffset, Timelike};
use eyre::{Report, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::EmailConfig;
use crate::domain::RelativeDate;
use crate::schedule::Schedule;

pub struct EmailClient(Option<EmailConfig>);

impl EmailClient {
    pub fn new(config: &Option<EmailConfig>) -> Self {
        Self(config.clone())
    }

    pub fn send_schedule(&self, date: RelativeDate, schedule: &Schedule) -> Result<()> {
        let subject = format!("Aikataulu {}", date.format("%d.%m.%Y"));
        let mut body: Vec<String> = Vec::new();

        for pin in &schedule.pins {
            let ranges = to_ranges(&pin.on_hours);
            body.push(format!(
                "{}: {} ({} h)\nKeskihinta: päällä {:.3}, pois {:.3}\n",
                pin.name,
                ranges,
                pin.on_hours.len(),
                pin.avg_price(&schedule.prices, true),
                pin.avg_price(&schedule.prices, false)
            ));
        }
        body.push(format!(
            "Vuorokauden keskihinta: {:.3}",
            schedule.avg_price()
        ));

        self.send(subject, body.join("\n"))
    }

    pub fn send_pin_state_change(&self, pins: &[(&str, u8, bool)], powered_on: bool) -> Result<()> {
        let subject = format!(
            "Tilamuutos{}",
            if powered_on { " (virta kytketty)" } else { "" }
        );
        let body = pins
            .iter()
            .map(|(name, pin, state)| {
                format!(
                    "{} (pinni {}): {}",
                    name,
                    pin,
                    if *state { "päällä" } else { "pois" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.send(subject, body)
    }

    pub fn send_error_making_tomorrows_schedule(&self, error: &Report) -> Result<()> {
        let subject = "Huomisen aikataulun laskeminen ei onnistunut".to_string();
        let body = format!("{:?}", error);
        self.send(subject, body)
    }

    pub fn send_error(&self, error: &Report) -> Result<()> {
        let subject = "Odottamaton virhe".to_string();
        let body = format!("{:?}", error);
        self.send(subject, body)
    }

    fn send(&self, subject: String, body: String) -> Result<()> {
        if let Some(config) = &self.0 {
            let message = Message::builder().from(config.from.parse()?);
            let message = config
                .to
                .iter()
                .fold(message, |acc, to| acc.to(to.parse().unwrap()));
            let message = message
                .subject(subject.clone())
                .header(ContentType::TEXT_PLAIN)
                .body(body.clone())?;

            let transport = SmtpTransport::relay(&config.server)?
                .credentials(Credentials::new(
                    config.username.clone(),
                    config.password.clone(),
                ))
                .build();

            transport.send(&message)?;
        }
        println!("Subject: {}", subject);
        println!();
        println!("{}", body);
        println!("--------------------------------------------");
        Ok(())
    }
}

/// Assumes that hours is ordered
fn to_ranges(hours: &[DateTime<FixedOffset>]) -> String {
    if hours.is_empty() {
        return String::new();
    }

    let mut ranges = Vec::new();

    let mut start = hours[0];
    let mut end = hours[0];

    for entry in hours.iter().skip(1) {
        let hour = *entry;
        if hour == end + Duration::hours(1) {
            end = hour;
        } else {
            ranges.push(format!("{:02}:00-{:02}:59", start.hour(), end.hour()));
            start = hour;
            end = hour;
        }
    }

    ranges.push(format!("{:02}:00-{:02}:59", start.hour(), end.hour()));
    ranges.join(", ")
}
