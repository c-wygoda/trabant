//! Mathematical utilities for SGP4 calculations
//!
//! This module provides mathematical functions commonly used in orbital mechanics
//! and SGP4 propagation, with proper handling of edge cases and numerical precision.

use std::f64::consts::PI;
use super::constants::{TWO_PI, DEG_TO_RAD, RAD_TO_DEG};

/// Convert degrees to radians
/// 
/// # Arguments
/// * `degrees` - Angle in degrees
/// 
/// # Returns
/// Angle in radians
/// 
/// # Examples
/// ```
/// use trabant::sgp4::math::degrees_to_radians;
/// assert!((degrees_to_radians(180.0) - std::f64::consts::PI).abs() < 1e-12);
/// assert!((degrees_to_radians(90.0) - std::f64::consts::PI / 2.0).abs() < 1e-12);
/// ```
pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * DEG_TO_RAD
}

/// Convert radians to degrees
/// 
/// # Arguments
/// * `radians` - Angle in radians
/// 
/// # Returns
/// Angle in degrees
/// 
/// # Examples
/// ```
/// use trabant::sgp4::math::radians_to_degrees;
/// assert!((radians_to_degrees(std::f64::consts::PI) - 180.0).abs() < 1e-12);
/// assert!((radians_to_degrees(std::f64::consts::PI / 2.0) - 90.0).abs() < 1e-12);
/// ```
pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * RAD_TO_DEG
}

/// Normalize angle to [0, 2π) range
/// 
/// # Arguments
/// * `angle` - Angle in radians
/// 
/// # Returns
/// Normalized angle in [0, 2π) range
/// 
/// # Examples
/// ```
/// use trabant::sgp4::math::normalize_angle;
/// assert!((normalize_angle(3.0 * std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-12);
/// assert!((normalize_angle(-std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-12);
/// ```
pub fn normalize_angle(angle: f64) -> f64 {
    let mut normalized = angle;
    while normalized < 0.0 {
        normalized += TWO_PI;
    }
    while normalized >= TWO_PI {
        normalized -= TWO_PI;
    }
    normalized
}

/// Compute angle modulo 2π (equivalent to normalize_angle but using fmod)
/// 
/// # Arguments
/// * `angle` - Angle in radians
/// 
/// # Returns
/// Angle reduced to [0, 2π) range
pub fn mod_2pi(angle: f64) -> f64 {
    let reduced = angle % TWO_PI;
    if reduced < 0.0 {
        reduced + TWO_PI
    } else {
        reduced
    }
}

/// Normalize angle to [-π, π] range
/// 
/// # Arguments
/// * `angle` - Angle in radians
/// 
/// # Returns
/// Normalized angle in [-π, π] range
pub fn normalize_angle_symmetric(angle: f64) -> f64 {
    let mut normalized = normalize_angle(angle);
    if normalized > PI {
        normalized -= TWO_PI;
    }
    normalized
}

/// Compute the fractional part of a number
/// 
/// # Arguments
/// * `x` - Input number
/// 
/// # Returns
/// Fractional part (x - floor(x))
pub fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Safe inverse cosine with bounds checking
/// 
/// # Arguments
/// * `x` - Input value
/// 
/// # Returns
/// arccos(x) with x clamped to [-1, 1]
pub fn acos_safe(x: f64) -> f64 {
    x.max(-1.0).min(1.0).acos()
}

/// Safe inverse sine with bounds checking
/// 
/// # Arguments
/// * `x` - Input value
/// 
/// # Returns
/// arcsin(x) with x clamped to [-1, 1]
pub fn asin_safe(x: f64) -> f64 {
    x.max(-1.0).min(1.0).asin()
}

/// Compute magnitude of a 3D vector
/// 
/// # Arguments
/// * `vector` - 3D vector as [x, y, z]
/// 
/// # Returns
/// Magnitude of the vector
pub fn magnitude(vector: &[f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

/// Normalize a 3D vector to unit length
/// 
/// # Arguments
/// * `vector` - 3D vector as [x, y, z]
/// 
/// # Returns
/// Unit vector in same direction, or [0, 0, 0] if input is zero vector
pub fn normalize_vector(vector: &[f64; 3]) -> [f64; 3] {
    let mag = magnitude(vector);
    if mag > super::constants::EPSILON {
        [vector[0] / mag, vector[1] / mag, vector[2] / mag]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Compute dot product of two 3D vectors
/// 
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
/// 
/// # Returns
/// Dot product a · b
pub fn dot_product(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Compute cross product of two 3D vectors
/// 
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
/// 
/// # Returns
/// Cross product a × b
pub fn cross_product(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_angle_conversions() {
        assert!((degrees_to_radians(180.0) - PI).abs() < 1e-12);
        assert!((degrees_to_radians(90.0) - PI / 2.0).abs() < 1e-12);
        assert!((degrees_to_radians(0.0)).abs() < 1e-12);
        
        assert!((radians_to_degrees(PI) - 180.0).abs() < 1e-12);
        assert!((radians_to_degrees(PI / 2.0) - 90.0).abs() < 1e-12);
        assert!((radians_to_degrees(0.0)).abs() < 1e-12);
    }

    #[test]
    fn test_angle_normalization() {
        // Test normalize_angle (0 to 2π)
        assert!((normalize_angle(3.0 * PI) - PI).abs() < 1e-12);
        assert!((normalize_angle(-PI) - PI).abs() < 1e-12);
        assert!((normalize_angle(0.0)).abs() < 1e-12);
        assert!((normalize_angle(TWO_PI) - 0.0).abs() < 1e-12);
        
        // Test mod_2pi
        assert!((mod_2pi(3.0 * PI) - PI).abs() < 1e-12);
        assert!((mod_2pi(-PI) - PI).abs() < 1e-12);
        
        // Test symmetric normalization (-π to π)
        assert!((normalize_angle_symmetric(3.0 * PI) - PI).abs() < 1e-12);
        // -π normalizes to π in the symmetric range
        assert!((normalize_angle_symmetric(-PI) - PI).abs() < 1e-12);
    }

    #[test]
    fn test_safe_inverse_trig() {
        assert!((acos_safe(2.0) - 0.0).abs() < 1e-12);  // Should clamp to 1.0
        assert!((acos_safe(-2.0) - PI).abs() < 1e-12);  // Should clamp to -1.0
        assert!((acos_safe(0.0) - PI / 2.0).abs() < 1e-12);
        
        assert!((asin_safe(2.0) - PI / 2.0).abs() < 1e-12);  // Should clamp to 1.0
        assert!((asin_safe(-2.0) - (-PI / 2.0)).abs() < 1e-12);  // Should clamp to -1.0
    }

    #[test]
    fn test_vector_operations() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        
        assert!((magnitude(&a) - 1.0).abs() < 1e-12);
        assert!((dot_product(&a, &b) - 0.0).abs() < 1e-12);
        
        let cross = cross_product(&a, &b);
        assert!((cross[0] - 0.0).abs() < 1e-12);
        assert!((cross[1] - 0.0).abs() < 1e-12);
        assert!((cross[2] - 1.0).abs() < 1e-12);
        
        let unit = normalize_vector(&[3.0, 4.0, 0.0]);
        assert!((magnitude(&unit) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_frac() {
        assert!((frac(3.7) - 0.7).abs() < 1e-12);
        assert!((frac(-1.3) - 0.7).abs() < 1e-12);
        assert!((frac(2.0) - 0.0).abs() < 1e-12);
    }
}