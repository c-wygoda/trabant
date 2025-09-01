//! Observer location and topocentric coordinate calculations
//!
//! This module provides functionality for:
//! - Converting geodetic coordinates (latitude, longitude, altitude) to ECEF
//! - Calculating topocentric coordinates and look angles
//! - Observer location handling for satellite pass predictions
//!
//! The module uses WGS84 ellipsoid parameters for accurate Earth modeling
//! and supports altitude references based on the WGS84 ellipsoid.

pub mod geodetic;

pub use geodetic::{GeodeticCoordinate, EcefCoordinate, Wgs84, geodetic_to_ecef};