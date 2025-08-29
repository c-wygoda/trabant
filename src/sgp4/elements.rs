use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete orbital elements structure for SGP4 propagation
/// 
/// This structure contains all the orbital elements needed for satellite
/// propagation using the SGP4 algorithm. It supports conversion from both
/// TLE and OMM formats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbitalElements {
    /// Satellite name
    pub name: String,
    
    /// NORAD catalog number
    pub norad_id: u32,
    
    /// International designator
    pub international_designator: String,
    
    /// Epoch time (UTC)
    pub epoch: DateTime<Utc>,
    
    /// First derivative of mean motion (rev/day²)
    pub mean_motion_dot: f64,
    
    /// Second derivative of mean motion (rev/day³)
    pub mean_motion_ddot: f64,
    
    /// B* drag term (1/earth radii)
    pub bstar: f64,
    
    /// Ephemeris type (always 0 for SGP4)
    pub ephemeris_type: u32,
    
    /// Element set number
    pub element_set_number: u32,
    
    /// Inclination (degrees)
    pub inclination: f64,
    
    /// Right ascension of ascending node (degrees)
    pub raan: f64,
    
    /// Eccentricity (dimensionless)
    pub eccentricity: f64,
    
    /// Argument of perigee (degrees)
    pub argument_of_perigee: f64,
    
    /// Mean anomaly (degrees)
    pub mean_anomaly: f64,
    
    /// Mean motion (rev/day)
    pub mean_motion: f64,
    
    /// Revolution number at epoch
    pub revolution_number: u32,
    
    /// Classification (U, C, S)
    pub classification: char,
}

impl OrbitalElements {
    /// Validate orbital element ranges
    pub fn validate(&self) -> Result<(), String> {
        if self.inclination < 0.0 || self.inclination > 180.0 {
            return Err(format!("Invalid inclination: {}", self.inclination));
        }
        
        if self.eccentricity < 0.0 || self.eccentricity >= 1.0 {
            return Err(format!("Invalid eccentricity: {}", self.eccentricity));
        }
        
        if self.mean_motion <= 0.0 {
            return Err(format!("Invalid mean motion: {}", self.mean_motion));
        }
        
        Ok(())
    }
}