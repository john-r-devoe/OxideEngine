use std::collections::VecDeque;

use pyo3::{exceptions::PyValueError, prelude::*};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;
use csv::Reader;

use crate::data::{bar::Bar, ticker::Ticker};


#[derive(Debug, Deserialize)]
pub struct CSVStooq {
    #[serde(rename = "<TICKER>")]
    pub ticker: String,
    #[serde(rename = "<PER>")]
    pub per: f64,
    #[serde(rename = "<DATE>")]
    pub date: String,
    #[serde(rename = "<TIME>")]
    pub time: String,
    #[serde(rename = "<OPEN>")]
    pub open: f64,
    #[serde(rename = "<HIGH>")]
    pub high: f64,
    #[serde(rename = "<LOW>")]
    pub low: f64,
    #[serde(rename = "<CLOSE>")]
    pub close: f64,
    #[serde(rename = "<VOL>")]
    pub vol: f64,
    #[serde(rename = "<OPENINT>")]
    pub openint: f64,
}

#[pyfunction]
#[pyo3(signature = (csv_path, symbol = None))]
pub fn from_csv(csv_path: &str, symbol: Option<&str>) -> PyResult<Ticker> {
    let mut reader = Reader::from_path(csv_path)
        .map_err(|err| PyValueError::new_err(format!("Invalid CSV path: {err}")))?;
    
    let csv_stooq: Vec<CSVStooq> = reader.deserialize::<CSVStooq>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| PyValueError::new_err(format!("Invalid CSV format: {err}")))?;

    let mut bars: VecDeque<Bar> = VecDeque::new();
    for item in &csv_stooq {
        let d = NaiveDate::from_ymd_opt(item.date[0..4].parse().unwrap(), item.date[4..6].parse().unwrap(), item.date[6..8].parse().unwrap()).unwrap();
        let t = NaiveTime::from_hms_opt(item.time[0..2].parse().unwrap(), item.time[2..4].parse().unwrap(), item.time[4..6].parse().unwrap()).unwrap();
        let dt = NaiveDateTime::new(d, t);
        bars.push_front(Bar {
            datetime: dt,
            open: item.open,
            high: item.high,
            close: item.close,
            low: item.low,
            vol: item.vol,
        });

    }
    
    Ok(Ticker {
        symbol: symbol.unwrap_or(&csv_stooq[0].ticker.to_string()).to_string(),
        bars
    })
}
