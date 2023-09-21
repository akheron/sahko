use crate::schedule::Hour;
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::ops::RangeInclusive;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Descriptive name for what is being controlled
    pub name: String,

    /// Pin to control
    pub pin: u8,

    /// Time range during which to schedule the `on_duration` period
    pub between: RangeInclusive<Hour>,

    /// Always on (up to `max_on_hours`) if price is under this limit
    pub low_limit: Option<f64>,

    /// Always off if price is over this limit
    pub high_limit: Option<f64>,

    /// Minimum duration to keep the switch on if price is under `high_limit`
    pub min_on_hours: u32,

    /// Maximum duration to keep the switch on even if the price is under `low_limit`
    pub max_on_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP server address
    pub server: String,

    /// SMTP username
    pub username: String,

    /// SMTP password
    pub password: String,

    /// From address
    pub from: String,

    /// To addresses
    pub to: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub schedules: Vec<ScheduleConfig>,
    pub email: Option<EmailConfig>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).wrap_err("Failed to open config.json")?;
        serde_json::from_reader(file).wrap_err("Failed to parse config.json")
    }
}
