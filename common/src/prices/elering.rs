use crate::domain::RelativeDate;
use crate::prices::{round_price, Price};
use chrono::{Local, TimeZone};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use std::collections::HashMap;

pub struct EleringPriceClient(reqwest::blocking::Client);

impl EleringPriceClient {
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
            .get("https://dashboard.elering.ee/api/nps/price")
            .query(&[("start", start.to_rfc3339())])
            .query(&[("end", end.to_rfc3339())])
            .send()
            .wrap_err_with(|| "Unable to request spot prices")?
            .json::<EleringResponse>()
            .wrap_err_with(|| "Unable to parse spot prices")?;

        if !response.success {
            return Err(eyre!("Elering API returned error"));
        }

        Ok(response
            .data
            .iter()
            .filter_map(
                |(key, prices)| {
                    if key == "fi" {
                        Some(prices)
                    } else {
                        None
                    }
                },
            )
            .flat_map(|prices| prices.iter())
            .map(|price| {
                let c_per_kwh = price.price / 10.0; // €/MWh to cents/kWh
                Price {
                    validity: Local
                        .timestamp_opt(price.timestamp as i64, 0)
                        .unwrap()
                        .fixed_offset(),
                    price: if c_per_kwh > 0.0 {
                        // Add VAT
                        round_price(c_per_kwh * VAT)
                    } else {
                        // No VAT for negative prices
                        c_per_kwh
                    },
                }
            })
            .collect())
    }
}

const VAT: f64 = 1.255;

#[derive(Deserialize, Debug)]
struct EleringResponse {
    success: bool,
    // Keys: ee, fi, lt, lv
    data: HashMap<String, Vec<SpotPrice>>,
}

#[derive(Deserialize, Debug)]
struct SpotPrice {
    timestamp: u32, // unix timestamp
    price: f64,     // €/MWh
}
