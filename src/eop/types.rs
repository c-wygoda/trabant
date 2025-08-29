use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors that can occur during EOP operations
#[derive(Error, Debug)]
pub enum EopError {
    #[error("Date {date} is outside available EOP data range ({start} to {end})")]
    DateOutOfRange {
        date: NaiveDate,
        start: NaiveDate,
        end: NaiveDate,
    },
    
    #[error("No EOP data available for interpolation")]
    NoDataAvailable,
    
    #[error("Invalid EOP value: {field} = {value} (outside reasonable range)")]
    InvalidValue {
        field: String,
        value: f64,
    },
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Earth Orientation Parameters for a specific date
/// 
/// Contains all EOP values needed for coordinate transformations,
/// including polar motion, UT1-UTC corrections, and optional
/// nutation/CIP corrections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EopData {
    /// Modified Julian Date
    pub mjd: i64,
    
    /// X component of polar motion (arcseconds)
    pub x_pole: f64,
    
    /// Y component of polar motion (arcseconds)
    pub y_pole: f64,
    
    /// UT1 - UTC time difference (seconds)
    pub ut1_utc: f64,
    
    /// Length of day correction (seconds)
    pub lod: f64,
    
    /// X component of Celestial Intermediate Pole (CIP) offset (milliarcseconds)
    pub dx_cip: Option<f64>,
    
    /// Y component of Celestial Intermediate Pole (CIP) offset (milliarcseconds)
    pub dy_cip: Option<f64>,
    
    /// Nutation correction in longitude (milliarcseconds)
    pub dpsi: Option<f64>,
    
    /// Nutation correction in obliquity (milliarcseconds)
    pub deps: Option<f64>,
}

impl EopData {
    /// Create new EOP data with basic validation
    pub fn new(
        mjd: i64,
        x_pole: f64,
        y_pole: f64,
        ut1_utc: f64,
        lod: f64,
    ) -> Result<Self, EopError> {
        let data = EopData {
            mjd,
            x_pole,
            y_pole,
            ut1_utc,
            lod,
            dx_cip: None,
            dy_cip: None,
            dpsi: None,
            deps: None,
        };
        
        data.validate()?;
        Ok(data)
    }
    
    /// Validate EOP values are within reasonable ranges
    pub fn validate(&self) -> Result<(), EopError> {
        // Polar motion should be within ±1 arcsecond typically
        if self.x_pole.abs() > 2.0 {
            return Err(EopError::InvalidValue {
                field: "x_pole".to_string(),
                value: self.x_pole,
            });
        }
        
        if self.y_pole.abs() > 2.0 {
            return Err(EopError::InvalidValue {
                field: "y_pole".to_string(),
                value: self.y_pole,
            });
        }
        
        // UT1-UTC should be within ±1 second typically
        if self.ut1_utc.abs() > 1.5 {
            return Err(EopError::InvalidValue {
                field: "ut1_utc".to_string(),
                value: self.ut1_utc,
            });
        }
        
        // LOD should be small (few milliseconds)
        if self.lod.abs() > 0.01 {
            return Err(EopError::InvalidValue {
                field: "lod".to_string(),
                value: self.lod,
            });
        }
        
        Ok(())
    }
    
    /// Convert to NaiveDate
    pub fn to_date(&self) -> NaiveDate {
        super::mjd_to_date(self.mjd)
    }
}

/// Cache for EOP data with interpolation capabilities
/// 
/// Provides efficient lookup and linear interpolation of EOP values
/// for any date within the available range. Uses BTreeMap for O(log n)
/// lookups and maintains sorted order for interpolation.
pub struct EopCache {
    /// EOP data indexed by Modified Julian Date
    data: BTreeMap<i64, EopData>,
    
    /// Earliest available date for quick bounds checking
    min_mjd: i64,
    
    /// Latest available date for quick bounds checking
    max_mjd: i64,
}

impl EopCache {
    /// Create new empty EOP cache
    pub fn new() -> Self {
        EopCache {
            data: BTreeMap::new(),
            min_mjd: i64::MAX,
            max_mjd: i64::MIN,
        }
    }
    
    /// Add EOP data to cache
    pub fn insert(&mut self, eop_data: EopData) {
        let mjd = eop_data.mjd;
        self.min_mjd = self.min_mjd.min(mjd);
        self.max_mjd = self.max_mjd.max(mjd);
        self.data.insert(mjd, eop_data);
    }
    
    /// Get EOP data for exact MJD (no interpolation)
    pub fn get_exact(&self, mjd: i64) -> Option<&EopData> {
        self.data.get(&mjd)
    }
    
    /// Get interpolated EOP data for any date within range
    pub fn get_interpolated(&self, date: NaiveDate) -> Result<EopData, EopError> {
        let mjd = super::date_to_mjd(date);
        self.get_interpolated_mjd(mjd)
    }
    
    /// Get interpolated EOP data for MJD
    pub fn get_interpolated_mjd(&self, mjd: i64) -> Result<EopData, EopError> {
        if self.data.is_empty() {
            return Err(EopError::NoDataAvailable);
        }
        
        // Check bounds
        if mjd < self.min_mjd || mjd > self.max_mjd {
            return Err(EopError::DateOutOfRange {
                date: super::mjd_to_date(mjd),
                start: super::mjd_to_date(self.min_mjd),
                end: super::mjd_to_date(self.max_mjd),
            });
        }
        
        // Check for exact match first
        if let Some(exact) = self.data.get(&mjd) {
            return Ok(exact.clone());
        }
        
        // Find surrounding data points for interpolation
        let mut before: Option<(i64, &EopData)> = None;
        let mut after: Option<(i64, &EopData)> = None;
        
        for (&data_mjd, data) in &self.data {
            if data_mjd <= mjd && (before.is_none() || data_mjd > before.unwrap().0) {
                before = Some((data_mjd, data));
            }
            if data_mjd >= mjd && (after.is_none() || data_mjd < after.unwrap().0) {
                after = Some((data_mjd, data));
            }
        }
        
        match (before, after) {
            (Some((mjd1, data1)), Some((mjd2, data2))) if mjd1 != mjd2 => {
                // Linear interpolation
                let t = (mjd - mjd1) as f64 / (mjd2 - mjd1) as f64;
                let interpolated = EopData {
                    mjd,
                    x_pole: data1.x_pole + t * (data2.x_pole - data1.x_pole),
                    y_pole: data1.y_pole + t * (data2.y_pole - data1.y_pole),
                    ut1_utc: data1.ut1_utc + t * (data2.ut1_utc - data1.ut1_utc),
                    lod: data1.lod + t * (data2.lod - data1.lod),
                    dx_cip: match (data1.dx_cip, data2.dx_cip) {
                        (Some(v1), Some(v2)) => Some(v1 + t * (v2 - v1)),
                        _ => None,
                    },
                    dy_cip: match (data1.dy_cip, data2.dy_cip) {
                        (Some(v1), Some(v2)) => Some(v1 + t * (v2 - v1)),
                        _ => None,
                    },
                    dpsi: match (data1.dpsi, data2.dpsi) {
                        (Some(v1), Some(v2)) => Some(v1 + t * (v2 - v1)),
                        _ => None,
                    },
                    deps: match (data1.deps, data2.deps) {
                        (Some(v1), Some(v2)) => Some(v1 + t * (v2 - v1)),
                        _ => None,
                    },
                };
                Ok(interpolated)
            }
            (Some((_, data)), _) => Ok(data.clone()),
            (_, Some((_, data))) => Ok(data.clone()),
            _ => Err(EopError::NoDataAvailable),
        }
    }
    
    /// Get date range covered by this cache
    pub fn date_range(&self) -> Option<(NaiveDate, NaiveDate)> {
        if self.data.is_empty() {
            None
        } else {
            Some((
                super::mjd_to_date(self.min_mjd),
                super::mjd_to_date(self.max_mjd),
            ))
        }
    }
    
    /// Number of data points in cache
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for EopCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_eop_data_creation() {
        let eop = EopData::new(59000, 0.1, 0.2, 0.05, 0.001).unwrap();
        assert_eq!(eop.mjd, 59000);
        assert_eq!(eop.x_pole, 0.1);
        assert_eq!(eop.y_pole, 0.2);
    }

    #[test]
    fn test_eop_data_validation() {
        // Valid data
        assert!(EopData::new(59000, 0.1, 0.2, 0.05, 0.001).is_ok());
        
        // Invalid polar motion
        assert!(EopData::new(59000, 3.0, 0.2, 0.05, 0.001).is_err());
        
        // Invalid UT1-UTC
        assert!(EopData::new(59000, 0.1, 0.2, 2.0, 0.001).is_err());
    }

    #[test]
    fn test_eop_cache_basic() {
        let mut cache = EopCache::new();
        assert!(cache.is_empty());
        
        let eop1 = EopData::new(59000, 0.1, 0.2, 0.05, 0.001).unwrap();
        let eop2 = EopData::new(59001, 0.2, 0.3, 0.06, 0.002).unwrap();
        
        cache.insert(eop1.clone());
        cache.insert(eop2.clone());
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get_exact(59000).unwrap(), &eop1);
        assert_eq!(cache.get_exact(59001).unwrap(), &eop2);
    }

    #[test]
    fn test_eop_cache_interpolation() {
        let mut cache = EopCache::new();
        
        let eop1 = EopData::new(59000, 0.1, 0.2, 0.05, 0.001).unwrap();
        let eop2 = EopData::new(59002, 0.3, 0.4, 0.07, 0.003).unwrap();
        
        cache.insert(eop1);
        cache.insert(eop2);
        
        // Interpolate at midpoint
        let interpolated = cache.get_interpolated_mjd(59001).unwrap();
        assert_eq!(interpolated.mjd, 59001);
        assert!((interpolated.x_pole - 0.2).abs() < 1e-10);
        assert!((interpolated.y_pole - 0.3).abs() < 1e-10);
        assert!((interpolated.ut1_utc - 0.06).abs() < 1e-10);
    }

    #[test]
    fn test_eop_cache_out_of_range() {
        let mut cache = EopCache::new();
        let eop = EopData::new(59000, 0.1, 0.2, 0.05, 0.001).unwrap();
        cache.insert(eop);
        
        assert!(cache.get_interpolated_mjd(58999).is_err());
        assert!(cache.get_interpolated_mjd(59001).is_err());
    }

    #[test]
    fn test_date_mjd_conversion() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let mjd = super::super::date_to_mjd(date);
        let converted_back = super::super::mjd_to_date(mjd);
        assert_eq!(date, converted_back);
    }
}