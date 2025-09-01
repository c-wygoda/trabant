//! Geodetic coordinate conversions using WGS84 ellipsoid

use crate::coordinates::Vector3;

/// WGS84 ellipsoid parameters
#[derive(Debug, Clone, Copy)]
pub struct Wgs84;

impl Wgs84 {
    /// Semi-major axis (equatorial radius) in meters
    pub const SEMI_MAJOR_AXIS: f64 = 6378137.0;
    
    /// Flattening factor
    pub const FLATTENING: f64 = 1.0 / 298.257223563;
    
    /// Semi-minor axis (polar radius) in meters
    pub const SEMI_MINOR_AXIS: f64 = Self::SEMI_MAJOR_AXIS * (1.0 - Self::FLATTENING);
    
    /// First eccentricity squared
    pub const ECCENTRICITY_SQUARED: f64 = 2.0 * Self::FLATTENING - Self::FLATTENING * Self::FLATTENING;

    /// Calculate the radius of curvature in the prime vertical (N)
    pub fn prime_vertical_radius(latitude_rad: f64) -> f64 {
        let sin_lat = latitude_rad.sin();
        Self::SEMI_MAJOR_AXIS / (1.0 - Self::ECCENTRICITY_SQUARED * sin_lat * sin_lat).sqrt()
    }
}

/// Geodetic coordinate (latitude, longitude, altitude)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticCoordinate {
    /// Latitude in degrees
    pub latitude_deg: f64,
    /// Longitude in degrees  
    pub longitude_deg: f64,
    /// Height above WGS84 ellipsoid in meters
    pub height_m: f64,
}

impl GeodeticCoordinate {
    /// Create new geodetic coordinate
    pub fn new(latitude_deg: f64, longitude_deg: f64, height_m: f64) -> Result<Self, GeodeticError> {
        if !(-90.0..=90.0).contains(&latitude_deg) {
            return Err(GeodeticError::InvalidLatitude(latitude_deg));
        }
        if !(-180.0..=180.0).contains(&longitude_deg) {
            return Err(GeodeticError::InvalidLongitude(longitude_deg));
        }
        
        Ok(GeodeticCoordinate {
            latitude_deg,
            longitude_deg, 
            height_m,
        })
    }

    /// Get latitude in radians
    pub fn latitude_rad(&self) -> f64 {
        self.latitude_deg.to_radians()
    }

    /// Get longitude in radians
    pub fn longitude_rad(&self) -> f64 {
        self.longitude_deg.to_radians()
    }
}

/// ECEF (Earth-Centered, Earth-Fixed) coordinate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcefCoordinate {
    /// Position vector in ECEF frame
    pub position: Vector3,
}

impl EcefCoordinate {
    /// Create ECEF coordinate from individual components
    pub fn from_components(x: f64, y: f64, z: f64) -> Self {
        EcefCoordinate {
            position: Vector3::new([x, y, z]),
        }
    }

    /// Get X component (meters)
    pub fn x(&self) -> f64 {
        self.position.data[0]
    }

    /// Get Y component (meters)
    pub fn y(&self) -> f64 {
        self.position.data[1]
    }

    /// Get Z component (meters)
    pub fn z(&self) -> f64 {
        self.position.data[2]
    }
}

/// Errors that can occur in geodetic conversions
#[derive(Debug, Clone, PartialEq)]
pub enum GeodeticError {
    /// Invalid latitude (must be -90° to +90°)
    InvalidLatitude(f64),
    /// Invalid longitude (must be -180° to +180°)
    InvalidLongitude(f64),
}

impl std::fmt::Display for GeodeticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeodeticError::InvalidLatitude(lat) => {
                write!(f, "Invalid latitude {:.6}°, must be in range [-90°, +90°]", lat)
            }
            GeodeticError::InvalidLongitude(lon) => {
                write!(f, "Invalid longitude {:.6}°, must be in range [-180°, +180°]", lon)
            }
        }
    }
}

impl std::error::Error for GeodeticError {}

/// Convert geodetic coordinates to ECEF coordinates using WGS84 ellipsoid
pub fn geodetic_to_ecef(geodetic: &GeodeticCoordinate) -> Result<EcefCoordinate, GeodeticError> {
    let lat_rad = geodetic.latitude_rad();
    let lon_rad = geodetic.longitude_rad();
    let height = geodetic.height_m;

    let cos_lat = lat_rad.cos();
    let sin_lat = lat_rad.sin();
    let cos_lon = lon_rad.cos();
    let sin_lon = lon_rad.sin();

    // Calculate prime vertical radius of curvature
    let n = Wgs84::prime_vertical_radius(lat_rad);

    // Calculate ECEF coordinates
    let x = (n + height) * cos_lat * cos_lon;
    let y = (n + height) * cos_lat * sin_lon;
    let z = (n * (1.0 - Wgs84::ECCENTRICITY_SQUARED) + height) * sin_lat;

    Ok(EcefCoordinate::from_components(x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geodetic_to_ecef_equator() {
        let geodetic = GeodeticCoordinate::new(0.0, 0.0, 0.0).unwrap();
        let ecef = geodetic_to_ecef(&geodetic).unwrap();
        
        // Should be at (semi-major axis, 0, 0)
        assert!((ecef.x() - Wgs84::SEMI_MAJOR_AXIS).abs() < 1.0);
        assert!(ecef.y().abs() < 1.0);
        assert!(ecef.z().abs() < 1.0);
    }

    #[test]
    fn test_geodetic_to_ecef_berlin() {
        let berlin = GeodeticCoordinate::new(52.52, 13.405, 34.0).unwrap();
        let ecef = geodetic_to_ecef(&berlin).unwrap();

        // Values should be reasonable for Berlin
        println!("Berlin ECEF: ({:.1}, {:.1}, {:.1})", ecef.x(), ecef.y(), ecef.z());
        
        // Basic sanity checks 
        assert!(ecef.x() > 3_000_000.0);
        assert!(ecef.y() > 800_000.0);
        assert!(ecef.z() > 5_000_000.0);
    }

    #[test]  
    fn test_geodetic_coordinate_creation() {
        // Valid coordinates
        let valid = GeodeticCoordinate::new(52.52, 13.405, 34.0).unwrap();
        assert_eq!(valid.latitude_deg, 52.52);
        assert_eq!(valid.longitude_deg, 13.405);
        assert_eq!(valid.height_m, 34.0);

        // Invalid latitude
        assert!(matches!(
            GeodeticCoordinate::new(91.0, 0.0, 0.0),
            Err(GeodeticError::InvalidLatitude(91.0))
        ));

        // Invalid longitude
        assert!(matches!(
            GeodeticCoordinate::new(0.0, 181.0, 0.0),
            Err(GeodeticError::InvalidLongitude(181.0))
        ));
    }

    #[test]
    fn test_wgs84_constants() {
        // Verify WGS84 constants match published values
        assert!((Wgs84::SEMI_MAJOR_AXIS - 6378137.0).abs() < 1e-10);
        assert!((Wgs84::FLATTENING - 1.0/298.257223563).abs() < 1e-12);
    }
}