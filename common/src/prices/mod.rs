mod elering;
mod porssisahko;

use crate::domain::RelativeDate;
use crate::prices::elering::EleringPriceClient;
use crate::prices::porssisahko::PorssisahkoPriceClient;
use chrono::{DateTime, FixedOffset};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

pub fn round_price(price: f64) -> f64 {
    (price * 1000.0).round() / 1000.0
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub validity: DateTime<FixedOffset>,
    pub price: f64,
}

pub struct PriceClient;

impl PriceClient {
    pub fn new() -> Self {
        Self
    }

    pub fn get_prices_for_date(&self, date: RelativeDate) -> Result<Vec<Price>> {
        log::info!("Getting prices for {:?} from elering", date);
        let elering_prices = EleringPriceClient::new().get_prices_for_date(date);
        if let Ok(elering_prices) = elering_prices {
            // DST transition day may have only 23 hours
            if elering_prices.len() >= 23 {
                return Ok(elering_prices);
            }
        }

        log::info!("Getting prices for {:?} from porssisahko.net", date);
        PorssisahkoPriceClient::new()
            .get_prices_for_date(date)
            .wrap_err("Unable to get prices from porssisahko.net API")
    }
}

impl Default for PriceClient {
    fn default() -> Self {
        Self::new()
    }
}
