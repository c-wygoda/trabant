//! Topocentric coordinate transformations and ENU frame calculations
//!
//! This module implements observer-relative coordinate systems including:
//! - ECEF to ENU (East-North-Up) transformations
//! - Local horizon coordinate system calculations
//! - Observer position transformation matrices
//! - Look angle (azimuth/elevation) computations
//!
//! All calculations use WGS84 ellipsoid parameters for precision.

use crate::coordinates::{Matrix3, Vector3};
use super::ObserverError;

/// WGS84 ellipsoid constants
pub const WGS84_A: f64 = 6378137.0;         // Semi-major axis (meters)
pub const WGS84_F: f64 = 1.0 / 298.257223563; // Flattening
pub const WGS84_E2: f64 = 2.0 * WGS84_F - WGS84_F * WGS84_F; // First eccentricity squared

/// Observer location in geodetic coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observer {
    /// Latitude in radians (positive = North)
    pub latitude_rad: f64,
    /// Longitude in radians (positive = East)  
    pub longitude_rad: f64,
    /// Altitude above WGS84 ellipsoid in meters
    pub altitude_m: f64,
}

impl Observer {
    /// Create new observer from degrees and meters
    pub fn new(latitude_deg: f64, longitude_deg: f64, altitude_m: f64) -> Result<Self, ObserverError> {
        if latitude_deg.abs() > 90.0 {
            return Err(ObserverError::InvalidCoordinates(
                format!("Invalid latitude: {} degrees", latitude_deg)
            ));
        }
        
        let longitude_deg = longitude_deg % 360.0;
        let longitude_deg = if longitude_deg > 180.0 {
            longitude_deg - 360.0
        } else if longitude_deg < -180.0 {
            longitude_deg + 360.0
        } else {
            longitude_deg
        };

        Ok(Observer {
            latitude_rad: latitude_deg.to_radians(),
            longitude_rad: longitude_deg.to_radians(),
            altitude_m,
        })
    }

    /// Convert observer geodetic coordinates to ECEF coordinates
    pub fn to_ecef(&self) -> Vector3 {
        let lat = self.latitude_rad;
        let lon = self.longitude_rad;
        let alt = self.altitude_m;

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let sin_lon = lon.sin();
        let cos_lon = lon.cos();

        // Prime vertical radius of curvature
        let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();

        let x = (n + alt) * cos_lat * cos_lon;
        let y = (n + alt) * cos_lat * sin_lon;
        let z = (n * (1.0 - WGS84_E2) + alt) * sin_lat;

        Vector3::new([x, y, z])
    }

    /// Get ENU transformation matrix for this observer location
    pub fn enu_matrix(&self) -> Matrix3 {
        enu_transformation_matrix(self.latitude_rad, self.longitude_rad)
    }
}

/// Topocentric coordinate frame relative to an observer
#[derive(Debug, Clone)]
pub struct TopocentricFrame {
    /// Observer location
    pub observer: Observer,
    /// ENU transformation matrix
    pub enu_matrix: Matrix3,
    /// Observer ECEF position
    pub observer_ecef: Vector3,
}

impl TopocentricFrame {
    /// Create new topocentric frame for observer
    pub fn new(observer: Observer) -> Self {
        let enu_matrix = observer.enu_matrix();
        let observer_ecef = observer.to_ecef();

        TopocentricFrame {
            observer,
            enu_matrix,
            observer_ecef,
        }
    }

    /// Transform ECEF position to ENU coordinates relative to observer
    pub fn ecef_to_enu(&self, ecef_pos: Vector3) -> Vector3 {
        ecef_to_enu(ecef_pos, &self.observer_ecef, &self.enu_matrix)
    }

    /// Calculate look angles (azimuth/elevation) to a target position
    pub fn calculate_look_angles(&self, target_ecef: Vector3) -> Result<LookAngles, ObserverError> {
        calculate_look_angles(&self.observer_ecef, target_ecef, &self.enu_matrix)
    }
}

/// Look angles from observer to target
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookAngles {
    /// Azimuth angle in radians (0 = North, π/2 = East, π = South, 3π/2 = West)
    pub azimuth_rad: f64,
    /// Elevation angle in radians (0 = horizon, π/2 = zenith, negative = below horizon)
    pub elevation_rad: f64,
    /// Range/distance in meters
    pub range_m: f64,
}

impl LookAngles {
    /// Get azimuth in degrees
    pub fn azimuth_deg(&self) -> f64 {
        self.azimuth_rad.to_degrees()
    }

    /// Get elevation in degrees  
    pub fn elevation_deg(&self) -> f64 {
        self.elevation_rad.to_degrees()
    }
}

/// Create ENU (East-North-Up) transformation matrix for given observer location
/// 
/// The ENU frame has:
/// - East axis: positive X
/// - North axis: positive Y  
/// - Up axis: positive Z (away from Earth center)
pub fn enu_transformation_matrix(latitude_rad: f64, longitude_rad: f64) -> Matrix3 {
    let sin_lat = latitude_rad.sin();
    let cos_lat = latitude_rad.cos();
    let sin_lon = longitude_rad.sin();
    let cos_lon = longitude_rad.cos();

    // ENU transformation matrix from ECEF coordinates
    // Each row represents the projection of ECEF (X,Y,Z) onto ENU (E,N,U) axes
    Matrix3::new([
        [-sin_lon, cos_lon, 0.0],                    // East direction
        [-sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat], // North direction  
        [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat],    // Up direction
    ])
}

/// Transform ECEF coordinates to ENU coordinates relative to observer
pub fn ecef_to_enu(ecef_pos: Vector3, observer_ecef: &Vector3, enu_matrix: &Matrix3) -> Vector3 {
    // Calculate relative position vector from observer to target
    let relative_pos = Vector3::new([
        ecef_pos.data[0] - observer_ecef.data[0],
        ecef_pos.data[1] - observer_ecef.data[1], 
        ecef_pos.data[2] - observer_ecef.data[2],
    ]);

    // Transform to ENU coordinates
    enu_matrix.transform_vector(relative_pos)
}

/// Calculate look angles (azimuth, elevation, range) from observer to target
pub fn calculate_look_angles(
    observer_ecef: &Vector3, 
    target_ecef: Vector3,
    enu_matrix: &Matrix3
) -> Result<LookAngles, ObserverError> {
    // Transform to ENU coordinates
    let enu_pos = ecef_to_enu(target_ecef, observer_ecef, enu_matrix);
    
    let east = enu_pos.data[0];
    let north = enu_pos.data[1]; 
    let up = enu_pos.data[2];

    // Calculate range (3D distance)
    let range_m = enu_pos.magnitude();
    
    if range_m == 0.0 {
        return Err(ObserverError::ComputationError(
            "Target position is at observer location".to_string()
        ));
    }

    // Calculate azimuth (angle from North towards East)
    // atan2(East, North) gives angle from North axis
    let azimuth_rad = east.atan2(north);
    
    // Normalize azimuth to [0, 2π)
    let azimuth_rad = if azimuth_rad < 0.0 {
        azimuth_rad + 2.0 * std::f64::consts::PI
    } else {
        azimuth_rad
    };

    // Calculate elevation (angle above horizon)
    let horizontal_range = (east * east + north * north).sqrt();
    let elevation_rad = up.atan2(horizontal_range);

    Ok(LookAngles {
        azimuth_rad,
        elevation_rad,
        range_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn test_observer_creation() {
        // Test valid observer
        let observer = Observer::new(52.52, 13.405, 34.0).unwrap();
        assert!((observer.latitude_rad - 52.52_f64.to_radians()).abs() < TOLERANCE);
        assert!((observer.longitude_rad - 13.405_f64.to_radians()).abs() < TOLERANCE);
        assert_eq!(observer.altitude_m, 34.0);

        // Test invalid latitude
        assert!(Observer::new(95.0, 0.0, 0.0).is_err());
        assert!(Observer::new(-95.0, 0.0, 0.0).is_err());

        // Test longitude normalization
        let observer = Observer::new(0.0, 370.0, 0.0).unwrap();
        assert!((observer.longitude_rad - 10.0_f64.to_radians()).abs() < TOLERANCE);
        
        let observer = Observer::new(0.0, -190.0, 0.0).unwrap();
        assert!((observer.longitude_rad - 170.0_f64.to_radians()).abs() < TOLERANCE);
    }

    #[test]
    fn test_observer_to_ecef() {
        // Test observer at equator, prime meridian
        let observer = Observer::new(0.0, 0.0, 0.0).unwrap();
        let ecef = observer.to_ecef();
        
        // Should be approximately on X-axis at Earth's equatorial radius
        assert!((ecef.data[0] - WGS84_A).abs() < 1.0, "X coordinate should be ~{}, got {}", WGS84_A, ecef.data[0]);
        assert!(ecef.data[1].abs() < 1.0, "Y coordinate should be ~0, got {}", ecef.data[1]);
        assert!(ecef.data[2].abs() < 1.0, "Z coordinate should be ~0, got {}", ecef.data[2]);

        // Test observer at North Pole  
        let observer = Observer::new(90.0, 0.0, 0.0).unwrap();
        let ecef = observer.to_ecef();
        
        // Should be on Z-axis at polar radius
        let expected_z = WGS84_A * (1.0 - WGS84_E2);
        assert!(ecef.data[0].abs() < 1.0, "X coordinate should be ~0, got {}", ecef.data[0]);
        assert!(ecef.data[1].abs() < 1.0, "Y coordinate should be ~0, got {}", ecef.data[1]);
        assert!((ecef.data[2] - expected_z).abs() < 25000.0, "Z coordinate should be ~{}, got {}", expected_z, ecef.data[2]);
    }

    #[test]
    fn test_enu_transformation_matrix() {
        // Test ENU matrix for equator, prime meridian
        let enu_matrix = enu_transformation_matrix(0.0, 0.0);
        
        // At (0°, 0°):
        // East = (0, 1, 0) in ECEF (Y-axis)
        // North = (0, 0, 1) in ECEF (Z-axis) 
        // Up = (1, 0, 0) in ECEF (X-axis)
        
        // Test East direction (should align with ECEF Y-axis)
        let ecef_y = Vector3::new([0.0, 1.0, 0.0]);
        let enu_result = enu_matrix.transform_vector(ecef_y);
        assert!((enu_result.data[0] - 1.0).abs() < TOLERANCE, "East component should be 1.0, got {}", enu_result.data[0]);
        assert!(enu_result.data[1].abs() < TOLERANCE, "North component should be 0.0, got {}", enu_result.data[1]);
        assert!(enu_result.data[2].abs() < TOLERANCE, "Up component should be 0.0, got {}", enu_result.data[2]);

        // Test North direction (should align with ECEF Z-axis)
        let ecef_z = Vector3::new([0.0, 0.0, 1.0]);
        let enu_result = enu_matrix.transform_vector(ecef_z);
        assert!(enu_result.data[0].abs() < TOLERANCE, "East component should be 0.0, got {}", enu_result.data[0]);
        assert!((enu_result.data[1] - 1.0).abs() < TOLERANCE, "North component should be 1.0, got {}", enu_result.data[1]);
        assert!(enu_result.data[2].abs() < TOLERANCE, "Up component should be 0.0, got {}", enu_result.data[2]);

        // Test Up direction (should align with ECEF X-axis) 
        let ecef_x = Vector3::new([1.0, 0.0, 0.0]);
        let enu_result = enu_matrix.transform_vector(ecef_x);
        assert!(enu_result.data[0].abs() < TOLERANCE, "East component should be 0.0, got {}", enu_result.data[0]);
        assert!(enu_result.data[1].abs() < TOLERANCE, "North component should be 0.0, got {}", enu_result.data[1]);
        assert!((enu_result.data[2] - 1.0).abs() < TOLERANCE, "Up component should be 1.0, got {}", enu_result.data[2]);
    }

    #[test]
    fn test_ecef_to_enu_transformation() {
        // Berlin observer location
        let observer = Observer::new(52.52, 13.405, 34.0).unwrap();
        let observer_ecef = observer.to_ecef();
        let enu_matrix = observer.enu_matrix();

        // Test target directly above observer (should be pure Up)
        let target_ecef = Vector3::new([
            observer_ecef.data[0] + 1000.0 * enu_matrix.data[2][0], // Add Up component in X
            observer_ecef.data[1] + 1000.0 * enu_matrix.data[2][1], // Add Up component in Y  
            observer_ecef.data[2] + 1000.0 * enu_matrix.data[2][2], // Add Up component in Z
        ]);

        let enu_pos = ecef_to_enu(target_ecef, &observer_ecef, &enu_matrix);
        
        assert!(enu_pos.data[0].abs() < TOLERANCE, "East should be ~0, got {}", enu_pos.data[0]);
        assert!(enu_pos.data[1].abs() < TOLERANCE, "North should be ~0, got {}", enu_pos.data[1]);
        assert!((enu_pos.data[2] - 1000.0).abs() < TOLERANCE, "Up should be ~1000, got {}", enu_pos.data[2]);
    }

    #[test]
    fn test_calculate_look_angles() {
        let observer = Observer::new(52.52, 13.405, 34.0).unwrap();
        let observer_ecef = observer.to_ecef();
        let enu_matrix = observer.enu_matrix();

        // Test target directly above observer
        let target_ecef = Vector3::new([
            observer_ecef.data[0] + 1000.0 * enu_matrix.data[2][0],
            observer_ecef.data[1] + 1000.0 * enu_matrix.data[2][1],
            observer_ecef.data[2] + 1000.0 * enu_matrix.data[2][2],
        ]);

        let angles = calculate_look_angles(&observer_ecef, target_ecef, &enu_matrix).unwrap();
        
        // Should point straight up (90° elevation)
        assert!((angles.elevation_rad - std::f64::consts::PI / 2.0).abs() < TOLERANCE, 
                "Elevation should be π/2, got {}", angles.elevation_rad);
        assert!((angles.range_m - 1000.0).abs() < TOLERANCE, 
                "Range should be 1000m, got {}", angles.range_m);
        // Azimuth is undefined for straight up, but should be valid number
        assert!(!angles.azimuth_rad.is_nan());

        // Test target to the North 
        let target_ecef = Vector3::new([
            observer_ecef.data[0] + 1000.0 * enu_matrix.data[1][0], // North direction
            observer_ecef.data[1] + 1000.0 * enu_matrix.data[1][1],
            observer_ecef.data[2] + 1000.0 * enu_matrix.data[1][2],
        ]);

        let angles = calculate_look_angles(&observer_ecef, target_ecef, &enu_matrix).unwrap();
        
        // Should point North (0° azimuth, 0° elevation)  
        assert!(angles.azimuth_rad.abs() < TOLERANCE || 
                (angles.azimuth_rad - 2.0 * std::f64::consts::PI).abs() < TOLERANCE,
                "Azimuth should be ~0, got {}", angles.azimuth_rad);
        assert!(angles.elevation_rad.abs() < TOLERANCE, 
                "Elevation should be ~0, got {}", angles.elevation_rad);
        assert!((angles.range_m - 1000.0).abs() < TOLERANCE, 
                "Range should be 1000m, got {}", angles.range_m);

        // Test target to the East
        let target_ecef = Vector3::new([
            observer_ecef.data[0] + 1000.0 * enu_matrix.data[0][0], // East direction
            observer_ecef.data[1] + 1000.0 * enu_matrix.data[0][1],
            observer_ecef.data[2] + 1000.0 * enu_matrix.data[0][2],
        ]);

        let angles = calculate_look_angles(&observer_ecef, target_ecef, &enu_matrix).unwrap();
        
        // Should point East (90° azimuth, 0° elevation)
        assert!((angles.azimuth_rad - std::f64::consts::PI / 2.0).abs() < TOLERANCE,
                "Azimuth should be π/2, got {}", angles.azimuth_rad);
        assert!(angles.elevation_rad.abs() < TOLERANCE,
                "Elevation should be ~0, got {}", angles.elevation_rad);
    }

    #[test]
    fn test_topocentric_frame() {
        let observer = Observer::new(52.52, 13.405, 34.0).unwrap();
        let frame = TopocentricFrame::new(observer);

        // Test that frame correctly stores observer data
        assert_eq!(frame.observer, observer);
        
        // Test ENU transformation through frame
        let target_ecef = Vector3::new([
            frame.observer_ecef.data[0] + 1000.0,
            frame.observer_ecef.data[1],
            frame.observer_ecef.data[2],
        ]);
        
        let enu_pos = frame.ecef_to_enu(target_ecef);
        let angles = frame.calculate_look_angles(target_ecef).unwrap();
        
        // Results should match direct calculations
        let direct_enu = ecef_to_enu(target_ecef, &frame.observer_ecef, &frame.enu_matrix);
        let direct_angles = calculate_look_angles(&frame.observer_ecef, target_ecef, &frame.enu_matrix).unwrap();
        
        assert_eq!(enu_pos, direct_enu);
        assert_eq!(angles.azimuth_rad, direct_angles.azimuth_rad);
        assert_eq!(angles.elevation_rad, direct_angles.elevation_rad);
        assert_eq!(angles.range_m, direct_angles.range_m);
    }

    #[test]
    fn test_look_angles_degrees() {
        let angles = LookAngles {
            azimuth_rad: std::f64::consts::PI / 2.0,  // 90 degrees
            elevation_rad: std::f64::consts::PI / 4.0, // 45 degrees
            range_m: 1000.0,
        };

        assert!((angles.azimuth_deg() - 90.0).abs() < TOLERANCE);
        assert!((angles.elevation_deg() - 45.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_error_handling() {
        let observer = Observer::new(52.52, 13.405, 34.0).unwrap();
        let observer_ecef = observer.to_ecef();
        let enu_matrix = observer.enu_matrix();

        // Test zero range error
        let result = calculate_look_angles(&observer_ecef, observer_ecef, &enu_matrix);
        assert!(result.is_err());
        match result {
            Err(ObserverError::ComputationError(_)) => {}, // Expected
            _ => panic!("Expected ComputationError for zero range"),
        }
    }
}