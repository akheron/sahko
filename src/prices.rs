use crate::domain::RelativeDate;
use crate::schedule::Hour;
use eyre::{ContextCompat, Result, WrapErr};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub validity: Hour,
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
            .enumerate()
            .map(|(hour, price)| {
                let start =
                    Hour::new(hour.try_into()?).wrap_err("Too many price entries per day")?;
                Ok(Price {
                    validity: start,
                    price: price.price,
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
                    price.price * VAT
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
    #[serde(rename = "PriceNoTax")]
    price: f64,
}
