//! Earth Orientation Parameters (EOP) handling
//!
//! This module provides functionality for parsing and interpolating Earth orientation 
//! parameters from IERS data. These parameters are essential for accurate coordinate
//! frame transformations between TEME, GCRS, and ITRS coordinate systems.

pub mod types;

pub use types::{EopData, EopCache, EopError};

use chrono::NaiveDate;

/// Convert Julian Date to Modified Julian Date (MJD)
pub fn jd_to_mjd(jd: f64) -> f64 {
    jd - 2400000.5
}

/// Convert Modified Julian Date to Julian Date
pub fn mjd_to_jd(mjd: f64) -> f64 {
    mjd + 2400000.5
}

/// Convert NaiveDate to Modified Julian Date
pub fn date_to_mjd(date: NaiveDate) -> i64 {
    // Reference: 1858-11-17 is MJD 0
    let mjd_epoch = NaiveDate::from_ymd_opt(1858, 11, 17).unwrap();
    date.signed_duration_since(mjd_epoch).num_days()
}

/// Convert Modified Julian Date to NaiveDate
pub fn mjd_to_date(mjd: i64) -> NaiveDate {
    let mjd_epoch = NaiveDate::from_ymd_opt(1858, 11, 17).unwrap();
    mjd_epoch + chrono::Duration::days(mjd)
}