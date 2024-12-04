use crate::domain::RelativeDate;
use crate::prices::Price;
use chrono::{DateTime, Local, Utc};
use eyre::{Result, WrapErr};
use serde::Deserialize;

pub struct PorssisahkoPriceClient(reqwest::blocking::Client);

impl PorssisahkoPriceClient {
    pub fn new() -> Self {
        Self(
            reqwest::blocking::ClientBuilder::new()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        )
    }

    pub fn get_prices_for_date(&self, date: RelativeDate) -> Result<Vec<Price>> {
        let (start, end) = date.to_start_and_end();
        let response = self
            .0
            .get("https://api.porssisahko.net/v1/latest-prices.json")
            .send()
            .wrap_err_with(|| "Unable to request spot prices")?
            .json::<PorssisahkoResponse>()
            .wrap_err_with(|| "Unable to parse spot prices")?;

        let mut prices = response
            .prices
            .into_iter()
            .filter(|price| start <= price.start_date && price.start_date < end)
            .map(|price| Price {
                validity: price.start_date.with_timezone(&Local).fixed_offset(),
                price: price.price,
            })
            .collect::<Vec<_>>();

        prices.sort_by_key(|price| price.validity);
        Ok(prices)
    }
}

#[derive(Deserialize, Debug)]
struct PorssisahkoResponse {
    prices: Vec<SpotPrice>,
}

#[derive(Deserialize, Debug)]
struct SpotPrice {
    #[serde(rename = "startDate")]
    start_date: DateTime<Utc>,
    price: f64, // c/kWh, includes VAT
}
