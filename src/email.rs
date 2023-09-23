use eyre::{Report, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::EmailConfig;
use crate::domain::RelativeDate;
use crate::schedule::{Hour, Schedule};

pub struct EmailClient(Option<EmailConfig>);

impl EmailClient {
    pub fn new(config: &Option<EmailConfig>) -> Self {
        Self(config.clone())
    }

    pub fn send_schedule(&self, date: RelativeDate, schedule: &Schedule) -> Result<()> {
        let subject = format!("Schedule for {}", date.format("%Y-%m-%d"));
        let body = schedule
            .pins
            .iter()
            .map(|schedule| format!("{}: {}", schedule.name, to_ranges(&schedule.on)))
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
        if let Some(config) = &self.0 {
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
        } else {
            println!("Subject: {}", subject);
            println!();
            println!("{}", body);
            println!("--------------------------------------------");
        }
        Ok(())
    }
}

/// Assumes that hours is ordered
fn to_ranges(hours: &[Hour]) -> String {
    if hours.is_empty() {
        return String::new();
    }

    let mut ranges = Vec::new();

    let mut start = hours[0].as_u32();
    let mut end = hours[0].as_u32();

    for entry in hours.iter().skip(1) {
        let hour = entry.as_u32();
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
