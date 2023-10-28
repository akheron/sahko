use crate::domain::RelativeDate;
use chrono::{DateTime, FixedOffset};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub validity: DateTime<FixedOffset>,
    pub price: f64,
}

pub struct PriceClient(reqwest::blocking::Client);

impl PriceClient {
    pub fn new() -> Self {
        Self(
            reqwest::blocking::ClientBuilder::new()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        )
    }

    fn get_prices(&self, url_suffix: &str) -> Result<Vec<Price>> {
        let url = format!("https://api.spot-hinta.fi/{}", url_suffix);
        self.0
            .get(&url)
            .send()
            .wrap_err_with(|| format!("Unable to request spot prices from {}", url))?
            .json::<Vec<SpotPrice>>()
            .wrap_err_with(|| format!("Unable to parse spot prices from {}", url))?
            .iter()
            .map(|price| {
                Ok(Price {
                    validity: price.date_time,
                    price: price.price * 100.0, // € to cents
                })
            })
            .collect()
    }

    pub fn get_prices_for_date(&self, date: RelativeDate) -> Result<Vec<Price>> {
        self.get_prices(match date {
            RelativeDate::Today => "Today",
            RelativeDate::Tomorrow => "DayForward",
        })?
        .iter()
        .map(|price| {
            Ok(Price {
                validity: price.validity,
                price: if price.price > 0.0 {
                    // Round to 3 decimal places
                    (price.price * VAT * 1000.0).round() / 1000.0
                } else {
                    price.price
                },
            })
        })
        .collect()
    }
}

impl Default for PriceClient {
    fn default() -> Self {
        Self::new()
    }
}

const VAT: f64 = 1.24;

#[derive(Deserialize, Debug)]
struct SpotPrice {
    #[serde(rename = "DateTime")]
    date_time: DateTime<FixedOffset>,

    #[serde(rename = "PriceNoTax")]
    price: f64,
}
