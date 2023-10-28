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
        let subject = format!("Aikataulu {}", date.format("%d.%m.%Y"));
        let mut body: Vec<String> = Vec::new();

        for pin in &schedule.pins {
            let ranges = to_ranges(&pin.on);
            let num_hours = pin.on.len();
            let avg_price = schedule
                .prices
                .iter()
                .filter(|price| pin.on.contains(&price.validity))
                .map(|price| price.price)
                .sum::<f64>()
                / num_hours as f64;
            body.push(format!(
                "{}: {} ({} h)\nKeskihinta: {:.3}\n",
                pin.name, ranges, num_hours, avg_price
            ));
        }
        body.push(format!(
            "Vuorokauden keskihinta: {:.3}",
            schedule.prices.iter().map(|price| price.price).sum::<f64>()
                / schedule.prices.len() as f64
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
