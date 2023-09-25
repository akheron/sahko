use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

use crate::config::ScheduleConfig;
use crate::prices::Price;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct Hour(u32);

impl Hour {
    pub fn new(hour: u32) -> Option<Self> {
        if hour < 24 {
            Some(Self(hour))
        } else {
            None
        }
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Hour {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(format!("Unable to convert {} to hour", value))
    }
}

impl Display for Hour {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:00", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinSchedule {
    pub name: String,
    pub pin: u8,
    pub on: Vec<Hour>,
}

impl PinSchedule {
    pub fn compute(config: &ScheduleConfig, prices: &[Price]) -> Self {
        // Filter out prices outside `between` or over `high_limit`
        let mut candidate_prices: Vec<Price> = prices
            .iter()
            .copied()
            .filter(|price| config.between.contains(&price.validity))
            .filter(|price| {
                if let Some(limit) = config.high_limit {
                    price.price < limit
                } else {
                    true
                }
            })
            .collect();

        // Sort by price
        candidate_prices.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        // Take all prices less than or equal to `low_limit`
        let (mut result, mut others): (Vec<Price>, Vec<Price>) =
            candidate_prices.iter().copied().partition(|price| {
                if let Some(limit) = config.low_limit {
                    price.price <= limit
                } else {
                    false
                }
            });

        // Truncate to `max_on_hours`
        if result.len() > config.max_on_hours as usize {
            result.truncate(config.max_on_hours as usize);
        }

        // Fill up to `min_on_hours`
        if result.len() < config.min_on_hours as usize {
            others.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            result.extend(
                others
                    .iter()
                    .take(config.min_on_hours as usize - result.len()),
            );
        }

        // Sort back to time order
        result.sort_by_key(|price| price.validity);

        Self {
            name: config.name.clone(),
            pin: config.pin,
            on: result.into_iter().map(|price| price.validity).collect(),
        }
    }

    pub fn is_on(&self, now: &NaiveDateTime) -> bool {
        let now_hour = now.hour();
        if let Some(hour) = Hour::new(now_hour) {
            self.on.iter().any(|entry| *entry == hour)
        } else {
            false
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule {
    pub pins: Vec<PinSchedule>,
    pub prices: Vec<Price>,
}

impl Schedule {
    pub fn compute(config: &[ScheduleConfig], prices: &[Price]) -> Self {
        Self {
            pins: config
                .iter()
                .map(|config| PinSchedule::compute(config, prices))
                .collect(),
            prices: prices.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prices::Price;
    use crate::schedule::{Hour, PinSchedule, ScheduleConfig};

    const DEFAULT_CONFIG: ScheduleConfig = ScheduleConfig {
        name: String::new(),
        pin: 0,
        between: Hour(0)..=Hour(23),
        low_limit: None,
        high_limit: None,
        min_on_hours: 1,
        max_on_hours: 1,
    };

    fn make_prices(price: f64) -> Vec<Price> {
        (0..=23)
            .map(|hour| Price {
                validity: Hour::new(hour).unwrap(),
                price,
            })
            .collect()
    }

    #[test]
    fn test_basic() {
        let prices = make_prices(0.0);

        let schedule = PinSchedule::compute(&DEFAULT_CONFIG, &prices);
        assert_eq!(schedule.on, vec![Hour(0)]);
    }

    #[test]
    fn test_low_limit() {
        let config = ScheduleConfig {
            low_limit: Some(0.0),
            max_on_hours: 3,
            ..DEFAULT_CONFIG
        };
        let prices = make_prices(0.0);

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on, vec![Hour(0), Hour(1), Hour(2)]);
    }

    #[test]
    fn test_between() {
        let config = ScheduleConfig {
            between: Hour(12)..=Hour(15),
            min_on_hours: 2,
            ..DEFAULT_CONFIG
        };
        let prices = make_prices(0.0);

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on, vec![Hour(12), Hour(13)]);
    }
    #[test]
    fn test_takes_lowest_prices_under_low_limit() {
        let config = ScheduleConfig {
            min_on_hours: 0,
            max_on_hours: 2,
            low_limit: Some(2.0),
            ..DEFAULT_CONFIG
        };
        let mut prices = vec![
            Price {
                validity: Hour(0),
                price: 1.5,
            },
            Price {
                validity: Hour(1),
                price: -1.0,
            },
            Price {
                validity: Hour(2),
                price: 1.0,
            },
            Price {
                validity: Hour(3),
                price: -2.0,
            },
        ];
        prices.extend(make_prices(5.0).iter().skip(4));

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on, vec![Hour(1), Hour(3)]);
    }
}
