//! SGP4 and WGS84 constants
//!
//! This module contains all the constants required for SGP4 propagation,
//! following Vallado's "Revisiting Spacetrack Report #3" and WGS84 specifications.

use std::f64::consts::PI;

// WGS84 Earth Parameters
/// Earth's equatorial radius in kilometers (WGS84)
pub const EARTH_RADIUS_KM: f64 = 6378.137;

/// Earth's gravitational parameter in km³/s² (WGS84)
pub const EARTH_GRAVITATIONAL_PARAMETER: f64 = 398600.5;

/// Earth's flattening factor (WGS84)
pub const EARTH_FLATTENING: f64 = 1.0 / 298.257223563;

// Gravitational Harmonics (WGS84)
/// Second zonal harmonic (J2)
pub const J2: f64 = 1.08262998905e-3;

/// Third zonal harmonic (J3)
pub const J3: f64 = -2.53215306e-6;

/// Fourth zonal harmonic (J4)
pub const J4: f64 = -1.61098761e-6;

// SGP4 Specific Constants (per Vallado)
/// Time units per minute
pub const TUMIN: f64 = 13.446839696312;

/// Earth angular velocity (radians per time unit)
pub const XKE: f64 = 0.0743669161331734132;

/// Derived constant CK2 = J2/2
pub const CK2: f64 = J2 / 2.0;

/// Derived constant CK4 = -3*J4/8
pub const CK4: f64 = -3.0 * J4 / 8.0;

/// Derived constant A3OVK2 = -J3/CK2
pub const A3OVK2: f64 = -J3 / CK2;

/// SGP4 specific constant QOMS2T
pub const QOMS2T: f64 = 1.88027916781e-9;

/// SGP4 specific constant S
pub const S_CONSTANT: f64 = 1.01222928;

// Mathematical Constants
/// Two times PI
pub const TWO_PI: f64 = 2.0 * PI;

/// PI divided by 180 (degrees to radians conversion factor)
pub const DEG_TO_RAD: f64 = PI / 180.0;

/// 180 divided by PI (radians to degrees conversion factor)
pub const RAD_TO_DEG: f64 = 180.0 / PI;

/// Tolerance for numerical computations
pub const EPSILON: f64 = 1e-12;

/// Maximum iterations for iterative algorithms
pub const MAX_ITERATIONS: usize = 10;

// Physical Constants
/// Speed of light in km/s
pub const SPEED_OF_LIGHT_KM_S: f64 = 299792.458;

/// Standard gravitational parameter for Sun (km³/s²)
pub const SUN_GRAVITATIONAL_PARAMETER: f64 = 1.32712442099e11;

/// Standard gravitational parameter for Moon (km³/s²)
pub const MOON_GRAVITATIONAL_PARAMETER: f64 = 4.9028000661e3;