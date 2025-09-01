//! Observer location handling and topocentric coordinate calculations
//!
//! This module provides functionality for:
//! - Converting geodetic coordinates (lat/lon/alt) to ECEF coordinates
//! - Topocentric coordinate transformations from observer perspective  
//! - ENU (East-North-Up) frame calculations
//! - Local horizon coordinate system computations
//! - Look angle calculations (azimuth and elevation)
//!
//! All calculations use WGS84 ellipsoid parameters and handle different
//! altitude references for precise satellite tracking applications.

pub mod topocentric;

pub use topocentric::{
    Observer, TopocentricFrame, enu_transformation_matrix,
    ecef_to_enu, calculate_look_angles, LookAngles,
};

/// Observer-related error types
#[derive(Debug, Clone, PartialEq)]
pub enum ObserverError {
    /// Invalid geodetic coordinates
    InvalidCoordinates(String),
    /// Computation error in coordinate transformation
    ComputationError(String),
}

impl std::fmt::Display for ObserverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserverError::InvalidCoordinates(msg) => write!(f, "Invalid coordinates: {}", msg),
            ObserverError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
        }
    }
}

impl std::error::Error for ObserverError {}