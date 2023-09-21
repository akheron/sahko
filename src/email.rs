use eyre::{Report, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::EmailConfig;
use crate::domain::Date;
use crate::prices::Price;
use crate::schedule::PinSchedule;

pub struct EmailClient(Option<EmailConfig>);

impl EmailClient {
    pub fn new(config: &Option<EmailConfig>) -> Self {
        Self(config.clone())
    }

    pub fn send_schedule(&self, date: Date, schedule: &[PinSchedule]) -> Result<()> {
        let subject = format!("Schedule for {}", date.format("%Y-%m-%d"));
        let body = schedule
            .iter()
            .map(|schedule| format!("{}: {}", schedule.name, to_ranges(&schedule.prices)))
            .collect::<Vec<_>>()
            .join("\n");
        self.send(subject, body)
    }

    pub fn send_pin_state_change(&self, name: &str, pin: u8, state: bool) -> Result<()> {
        let subject = format!("State change: {}", name);
        let body = format!(
            "{} (pin {}) is now {}",
            name,
            pin,
            if state { "on" } else { "off" }
        );
        self.send(subject, body)
    }

    pub fn send_error(&self, error: &Report) -> Result<()> {
        let subject = "Unexpected error".to_string();
        let body = format!("{:?}", error);
        self.send(subject, body)
    }

    fn send(&self, subject: String, body: String) -> Result<()> {
        let Some(config) = &self.0 else { return Ok(()) };

        let message = Message::builder().from(config.from.parse()?);
        let message = config
            .to
            .iter()
            .fold(message, |acc, to| acc.to(to.parse().unwrap()));
        let message = message
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        let transport = SmtpTransport::relay(&config.server)?
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .build();

        transport.send(&message)?;
        Ok(())
    }
}

/// Assumes that prices are ordered by validity
fn to_ranges(prices: &[Price]) -> String {
    if prices.is_empty() {
        return String::new();
    }

    let mut ranges = Vec::new();

    let mut start = prices[0].validity.as_u32();
    let mut end = prices[0].validity.as_u32();

    for price in prices.iter().skip(1) {
        let hour = price.validity.as_u32();
        if hour == end + 1 {
            end = hour;
        } else {
            ranges.push(format!("{:02}:00-{:02}:59", start, end));
            start = hour;
            end = hour;
        }
    }

    ranges.push(format!("{:02}:00-{:02}:59", start, end));
    ranges.join(", ")
}
