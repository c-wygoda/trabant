//! Coordinate frame transformations for satellite tracking
//!
//! This module provides transformations between different coordinate frames:
//! - TEME (True Equator, Mean Equinox) - SGP4 output frame
//! - GCRS (Geocentric Celestial Reference System) - Inertial frame
//! - ITRS (International Terrestrial Reference System) - Earth-fixed frame
//!
//! The transformations account for:
//! - Precession and nutation (TEME → GCRS)
//! - Earth rotation and polar motion (GCRS → ITRS)
//! - Earth Orientation Parameters (EOP) for accurate corrections

pub mod matrix;
pub mod teme_to_gcrs;
pub mod gcrs_to_itrs;

pub use matrix::{Matrix3, Vector3};
use crate::{StateVector, EopData};
use chrono::{DateTime, Utc};

/// Error types for coordinate transformations
#[derive(Debug, Clone, PartialEq)]
pub enum CoordinateError {
    /// EOP data not available or invalid
    EopDataError(String),
    /// Time conversion error
    TimeError(String),
    /// Numerical computation error
    ComputationError(String),
}

impl std::fmt::Display for CoordinateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoordinateError::EopDataError(msg) => write!(f, "EOP data error: {}", msg),
            CoordinateError::TimeError(msg) => write!(f, "Time conversion error: {}", msg),
            CoordinateError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
        }
    }
}

impl std::error::Error for CoordinateError {}

/// Coordinate transformation context
#[derive(Debug, Clone)]
pub struct CoordinateContext {
    pub time: DateTime<Utc>,
    pub eop: EopData,
    pub julian_date: f64,
    pub tt_mjd: f64,
    pub ut1_mjd: f64,
}

impl CoordinateContext {
    /// Create new coordinate context for given time and EOP data
    pub fn new(time: DateTime<Utc>, eop: EopData) -> Result<Self, CoordinateError> {
        let julian_date = time_to_julian_date(time);
        let tt_mjd = julian_date_to_tt_mjd(julian_date);
        let ut1_mjd = utc_to_ut1_mjd(time, eop.ut1_utc);
        
        Ok(CoordinateContext {
            time,
            eop,
            julian_date,
            tt_mjd,
            ut1_mjd,
        })
    }
}

/// Transform TEME state vector to GCRS
pub fn teme_to_gcrs(state: &StateVector, ctx: &CoordinateContext) -> Result<StateVector, CoordinateError> {
    teme_to_gcrs::transform(state, ctx)
}

/// Transform GCRS state vector to ITRS  
pub fn gcrs_to_itrs(state: &StateVector, ctx: &CoordinateContext) -> Result<StateVector, CoordinateError> {
    gcrs_to_itrs::transform(state, ctx)
}

/// Transform TEME state vector directly to ITRS (convenience function)
pub fn teme_to_itrs(state: &StateVector, ctx: &CoordinateContext) -> Result<StateVector, CoordinateError> {
    let gcrs_state = teme_to_gcrs(state, ctx)?;
    gcrs_to_itrs(&gcrs_state, ctx)
}

/// Convert DateTime<Utc> to Julian Date
pub fn time_to_julian_date(time: DateTime<Utc>) -> f64 {
    // Unix timestamp to Julian Date
    // Unix epoch (1970-01-01 00:00:00 UTC) = JD 2440587.5
    const UNIX_EPOCH_JD: f64 = 2440587.5;
    const SECONDS_PER_DAY: f64 = 86400.0;
    
    let unix_timestamp = time.timestamp() as f64 + time.timestamp_subsec_nanos() as f64 / 1e9;
    UNIX_EPOCH_JD + unix_timestamp / SECONDS_PER_DAY
}

/// Convert Julian Date to Terrestrial Time (TT) as Modified Julian Date
pub fn julian_date_to_tt_mjd(jd: f64) -> f64 {
    // TT = UTC + 32.184 + leap_seconds (simplified - using 37 leap seconds as of 2023)
    // For precision, should get actual leap seconds, but this is sufficient for current accuracy
    const TT_UTC_OFFSET: f64 = 69.184 / 86400.0; // ~69.184 seconds in days
    jd - 2400000.5 + TT_UTC_OFFSET // Convert to MJD and add TT offset
}

/// Convert UTC time to UT1 as Modified Julian Date using EOP UT1-UTC
pub fn utc_to_ut1_mjd(time: DateTime<Utc>, ut1_utc: f64) -> f64 {
    let jd = time_to_julian_date(time);
    let mjd = jd - 2400000.5;
    mjd + ut1_utc / 86400.0 // Add UT1-UTC correction in days
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_time_to_julian_date() {
        // Test Unix epoch: 1970-01-01 00:00:00 UTC = JD 2440587.5
        let epoch = Utc.timestamp_opt(0, 0).single().unwrap();
        let jd = time_to_julian_date(epoch);
        assert!((jd - 2440587.5).abs() < 1e-6, "Expected ~2440587.5, got {}", jd);
        
        // Test J2000 epoch: 2000-01-01 12:00:00 UTC = JD 2451545.0
        let j2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).single().unwrap();
        let jd_j2000 = time_to_julian_date(j2000);
        assert!((jd_j2000 - 2451545.0).abs() < 1e-6, "Expected ~2451545.0, got {}", jd_j2000);
    }

    #[test] 
    fn test_julian_date_to_tt_mjd() {
        let jd = 2451545.0; // J2000
        let tt_mjd = julian_date_to_tt_mjd(jd);
        let expected_mjd = 51544.5 + 69.184/86400.0;
        assert!((tt_mjd - expected_mjd).abs() < 1e-6, "TT conversion error");
    }

    #[test]
    fn test_utc_to_ut1_mjd() {
        let time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).single().unwrap();
        let ut1_utc = 0.3554; // Example UT1-UTC value in seconds
        let ut1_mjd = utc_to_ut1_mjd(time, ut1_utc);
        let expected = 51544.5 + ut1_utc / 86400.0;
        assert!((ut1_mjd - expected).abs() < 1e-9, "UT1 conversion error");
    }
}