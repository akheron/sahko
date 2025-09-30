use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::Write;

use crate::config::ScheduleConfig;
use crate::prices::Price;

#[derive(Debug, Serialize, Deserialize)]
pub struct PinSchedule {
    pub name: String,
    pub pin: u8,
    pub on_hours: Vec<DateTime<FixedOffset>>,
}

impl PinSchedule {
    pub fn compute(config: &ScheduleConfig, prices: &[Price]) -> Self {
        // Average hourly prices over each hour. Assumes that prices are in order.
        let mut hour_averages: Vec<Price> = Vec::new();
        let mut count = 0;
        for price in prices {
            if count == 0 {
                hour_averages.push(price.clone());
                count = 1;
            } else {
                let last_price = hour_averages.last_mut().unwrap();
                if last_price.validity.hour() == price.validity.hour() {
                    last_price.price += price.price;
                    count += 1;
                } else {
                    last_price.price /= count as f64;
                    hour_averages.push(Price {
                        // Use start of hour as validity just to be sure
                        validity: price.validity.with_minute(0).unwrap(),
                        price: price.price,
                    });
                    count = 1;
                }
            }
        }
        if let Some(last_price) = hour_averages.last_mut() {
            last_price.price /= count as f64;
        }

        // Filter out prices over `high_limit`
        hour_averages.retain(|price| {
            if let Some(limit) = config.high_limit {
                price.price < limit
            } else {
                true
            }
        });

        // Sort by price
        hour_averages.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        // Take all prices less than or equal to `low_limit`
        let (mut result, mut others): (Vec<Price>, Vec<Price>) =
            hour_averages.iter().copied().partition(|price| {
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

        // Remove ranges that occur in the middle of the day and are shorter than `min_consecutive_on_hours`
        if let Some(min_consecutive_on_hours) = config.min_consecutive_on_hours {
            let mut ranges: Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)> = Vec::new();
            for (i, price) in result.iter().enumerate() {
                if i == 0 {
                    ranges.push((price.validity, price.validity));
                } else {
                    let (_, end) = ranges.last_mut().unwrap();
                    if price.validity == *end + Duration::hours(1) {
                        *end = price.validity;
                    } else {
                        ranges.push((price.validity, price.validity));
                    }
                }
            }
            let too_short = ranges
                .into_iter()
                .filter(|(start, end)| {
                    start.hour() != 0
                        && end.hour() != 23
                        // + 1 because we use starts of hours but the length of an hour is 1 hour
                        && (*end - *start).num_hours() + 1 < min_consecutive_on_hours as i64
                })
                .collect::<Vec<_>>();

            result.retain(|price| {
                !too_short
                    .iter()
                    .any(|(start, end)| *start <= price.validity && price.validity <= *end)
            });
        }

        Self {
            name: config.name.clone(),
            pin: config.pin,
            on_hours: result.into_iter().map(|price| price.validity).collect(),
        }
    }

    pub fn is_on(&self, now: &DateTime<Local>) -> bool {
        self.on_hours
            .iter()
            .any(|entry| *entry <= *now && *now < *entry + Duration::hours(1))
    }

    pub fn avg_price(&self, prices: &[Price], on: bool) -> f64 {
        let num_hours = if on {
            self.on_hours.len()
        } else {
            24 - self.on_hours.len()
        };
        if num_hours == 0 {
            0.0
        } else {
            let selected_prices = prices
                .iter()
                .filter(|price| {
                    let is_on = self.on_hours.contains(&price.validity);
                    if on {
                        is_on
                    } else {
                        !is_on
                    }
                })
                .map(|price| price.price)
                .collect::<Vec<_>>();

            selected_prices.iter().sum::<f64>() / selected_prices.len() as f64
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

    pub fn avg_price(&self) -> f64 {
        // This assumes that all price spans are equal length and cover the whole day
        self.prices.iter().map(|price| price.price).sum::<f64>() / self.prices.len() as f64
    }

    pub fn avg_price_for_hour<Tz: TimeZone>(&self, hour: DateTime<Tz>) -> Option<f64> {
        let next_hour = hour.clone() + Duration::hours(1);
        let hour_prices = self
            .prices
            .iter()
            .filter(|price| hour <= price.validity && price.validity < next_hour)
            .map(|price| price.price)
            .collect::<Vec<_>>();
        if hour_prices.is_empty() {
            None
        } else {
            Some(hour_prices.iter().sum::<f64>() / hour_prices.len() as f64)
        }
    }

    pub fn load_for_date(date: NaiveDate) -> Option<Self> {
        let file = File::open(schedule_filename(date)).ok()?;
        serde_json::from_reader(file).ok()
    }

    pub fn write_to_file(&self, date: NaiveDate) -> std::io::Result<()> {
        create_dir_all(SCHEDULE_DIR_NAME)?;
        write!(
            File::create(schedule_filename(date))?,
            "{}",
            serde_json::to_string_pretty(self)?
        )
    }
}

const SCHEDULE_DIR_NAME: &str = "schedules";

fn schedule_filename(date: NaiveDate) -> String {
    format!(
        "{}/schedule_{}.json",
        SCHEDULE_DIR_NAME,
        date.format("%Y-%m-%d")
    )
}

#[cfg(test)]
mod tests {
    use crate::prices::Price;
    use crate::schedule::{PinSchedule, ScheduleConfig};
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use lazy_static::lazy_static;

    const DEFAULT_CONFIG: ScheduleConfig = ScheduleConfig {
        name: String::new(),
        pin: 0,
        low_limit: None,
        high_limit: None,
        min_on_hours: 1,
        max_on_hours: 1,
        min_consecutive_on_hours: None,
    };

    lazy_static! {
        static ref TZ: FixedOffset = FixedOffset::east_opt(2 * 3600).unwrap();
        static ref TODAY: NaiveDate = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
    }

    fn hour_dt(hour: u32) -> DateTime<FixedOffset> {
        TZ.from_local_datetime(&NaiveDateTime::new(
            *TODAY,
            NaiveTime::from_hms_opt(hour, 0, 0).unwrap(),
        ))
        .unwrap()
    }

    fn make_hourly_prices(price: f64) -> Vec<Price> {
        (0..=23)
            .map(|hour| Price {
                validity: hour_dt(hour),
                price,
            })
            .collect()
    }

    fn quarter_dt(hour: u32, quarter: u32) -> DateTime<FixedOffset> {
        TZ.from_local_datetime(&NaiveDateTime::new(
            *TODAY,
            NaiveTime::from_hms_opt(hour, 15 * quarter, 0).unwrap(),
        ))
        .unwrap()
    }

    fn make_quarterly_prices(price: f64) -> Vec<Price> {
        (0..=23)
            .flat_map(|hour| (0..=3).map(move |quarter| (hour, quarter)))
            .map(|(hour, quarter)| Price {
                validity: quarter_dt(hour, quarter),
                price,
            })
            .collect()
    }

    #[test]
    fn test_basic_hourly() {
        let prices = make_hourly_prices(0.0);

        let schedule = PinSchedule::compute(&DEFAULT_CONFIG, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(0)]);
    }

    #[test]
    fn test_basic_quarterly() {
        let prices = make_quarterly_prices(0.0);

        let schedule = PinSchedule::compute(&DEFAULT_CONFIG, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(0)]);
    }

    #[test]
    fn test_low_limit_hourly() {
        let config = ScheduleConfig {
            low_limit: Some(0.0),
            max_on_hours: 3,
            ..DEFAULT_CONFIG
        };
        let prices = make_hourly_prices(0.0);

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(0), hour_dt(1), hour_dt(2)]);
    }

    #[test]
    fn test_low_limit_quarterly() {
        let config = ScheduleConfig {
            low_limit: Some(0.0),
            max_on_hours: 3,
            ..DEFAULT_CONFIG
        };
        let prices = make_quarterly_prices(0.0);

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(0), hour_dt(1), hour_dt(2)]);
    }

    #[test]
    fn test_takes_lowest_prices_under_low_limit_hourly() {
        let config = ScheduleConfig {
            min_on_hours: 0,
            max_on_hours: 2,
            low_limit: Some(2.0),
            ..DEFAULT_CONFIG
        };
        let mut prices = vec![
            Price {
                validity: hour_dt(0),
                price: 1.5,
            },
            Price {
                validity: hour_dt(1),
                price: -1.0,
            },
            Price {
                validity: hour_dt(2),
                price: 1.0,
            },
            Price {
                validity: hour_dt(3),
                price: -2.0,
            },
        ];
        prices.extend(make_hourly_prices(5.0).iter().skip(4));

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(1), hour_dt(3)]);
    }

    #[test]
    fn test_takes_lowest_prices_under_low_limit_quarterly() {
        let config = ScheduleConfig {
            min_on_hours: 0,
            max_on_hours: 2,
            low_limit: Some(2.0),
            ..DEFAULT_CONFIG
        };
        let mut prices = vec![
            // Hour 0: avg 1.5
            Price {
                validity: quarter_dt(0, 0),
                price: 1.5,
            },
            Price {
                validity: quarter_dt(0, 1),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(0, 2),
                price: 1.5,
            },
            Price {
                validity: quarter_dt(0, 3),
                price: 2.0,
            },
            // Hour 1: avg -1.0
            Price {
                validity: quarter_dt(1, 0),
                price: -1.0,
            },
            Price {
                validity: quarter_dt(1, 1),
                price: -1.0,
            },
            Price {
                validity: quarter_dt(1, 2),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(1, 3),
                price: -3.0,
            },
            // Hour 2: avg 1.0
            Price {
                validity: quarter_dt(2, 0),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(2, 1),
                price: 2.0,
            },
            Price {
                validity: quarter_dt(2, 2),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(2, 3),
                price: 0.0,
            },
            // Hour 2: avg 0.0
            Price {
                validity: quarter_dt(3, 0),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(3, 1),
                price: -1.0,
            },
            Price {
                validity: quarter_dt(3, 2),
                price: 1.0,
            },
            Price {
                validity: quarter_dt(3, 3),
                price: -1.0,
            },
        ];
        prices.extend(make_quarterly_prices(5.0).iter().skip(4 * 4));

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(1), hour_dt(3)]);
    }

    #[test]
    fn does_not_include_too_short_ranges() {
        let config = ScheduleConfig {
            low_limit: Some(0.0),
            high_limit: Some(1.0),
            min_on_hours: 3,
            min_consecutive_on_hours: Some(2),
            ..DEFAULT_CONFIG
        };

        let mut prices = vec![
            Price {
                validity: hour_dt(0),
                price: 5.0,
            },
            // Too short, not kept --->
            Price {
                validity: hour_dt(1),
                price: 0.5,
            },
            // <---
            Price {
                validity: hour_dt(2),
                price: 5.0,
            },
            // Long enough, kept --->
            Price {
                validity: hour_dt(3),
                price: 0.5,
            },
            Price {
                validity: hour_dt(4),
                price: 0.5,
            },
            // <---
        ];
        prices.extend(make_hourly_prices(5.0).iter().skip(5));

        let schedule = PinSchedule::compute(&config, &prices);
        assert_eq!(schedule.on_hours, vec![hour_dt(3), hour_dt(4)]);
    }
}
