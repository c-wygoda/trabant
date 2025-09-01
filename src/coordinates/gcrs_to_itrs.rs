//! GCRS to ITRS coordinate frame transformation
//!
//! Transforms from Geocentric Celestial Reference System (GCRS) inertial frame
//! to International Terrestrial Reference System (ITRS) Earth-fixed frame using:
//! - Earth rotation (GMST/GAST)
//! - Polar motion corrections from EOP data

use crate::{StateVector, coordinates::{CoordinateContext, CoordinateError, matrix::{Matrix3, Vector3}}};

/// Transform state vector from GCRS to ITRS frame
pub fn transform(state: &StateVector, ctx: &CoordinateContext) -> Result<StateVector, CoordinateError> {
    // Compute transformation matrix from GCRS to ITRS
    let gcrs_to_itrs_matrix = compute_gcrs_to_itrs_matrix(ctx)?;
    
    // For velocity transformation, we need to account for Earth's rotation
    let omega_earth = earth_rotation_rate(); // rad/s
    let omega_vector = Vector3::new([0.0, 0.0, omega_earth]);
    
    // Transform position
    let pos_gcrs = Vector3::from(state.position);
    let pos_itrs = gcrs_to_itrs_matrix.transform_vector(pos_gcrs);
    
    // Transform velocity (includes Earth rotation effect)
    let vel_gcrs = Vector3::from(state.velocity);
    let vel_transformed = gcrs_to_itrs_matrix.transform_vector(vel_gcrs);
    
    // Add Earth rotation velocity correction: v_ITRS = v_transformed - ω × r_ITRS
    let rotation_correction = omega_vector.cross(&pos_itrs);
    let vel_itrs = vel_transformed + rotation_correction * (-1.0);
    
    Ok(StateVector {
        position: pos_itrs.into(),
        velocity: vel_itrs.into(),
    })
}

/// Compute the complete GCRS to ITRS transformation matrix
fn compute_gcrs_to_itrs_matrix(ctx: &CoordinateContext) -> Result<Matrix3, CoordinateError> {
    // Earth rotation matrix (GMST/GAST)
    let earth_rotation = earth_rotation_matrix(ctx.ut1_mjd)?;
    
    // Polar motion matrix
    let polar_motion = polar_motion_matrix(ctx.eop.x_pole, ctx.eop.y_pole)?;
    
    // Combined transformation: GCRS -> ITRS  
    // Apply polar motion first, then Earth rotation
    let combined = polar_motion * earth_rotation;
    
    Ok(combined)
}

/// Earth rotation matrix using Greenwich Mean Sidereal Time (GMST)
fn earth_rotation_matrix(ut1_mjd: f64) -> Result<Matrix3, CoordinateError> {
    // Compute Greenwich Mean Sidereal Time (GMST)
    let gmst = greenwich_mean_sidereal_time(ut1_mjd);
    
    // Earth rotation is a rotation around Z-axis by GMST angle
    Ok(Matrix3::rotation_z(-gmst)) // Negative because GCRS->ITRS
}

/// Polar motion matrix from EOP x_pole and y_pole
fn polar_motion_matrix(x_pole_arcsec: f64, y_pole_arcsec: f64) -> Result<Matrix3, CoordinateError> {
    // Convert from arcseconds to radians
    let arcsec_to_rad = std::f64::consts::PI / (180.0 * 3600.0);
    let x_p = x_pole_arcsec * arcsec_to_rad;
    let y_p = y_pole_arcsec * arcsec_to_rad;
    
    // Polar motion is a composition of rotations about Y and X axes
    // W(t) = R3(-s') R2(x_p) R1(y_p)
    // For simplicity, ignoring s' (TIO locator) which is typically very small
    
    let r_y = Matrix3::rotation_y(x_p);  // Rotation about Y-axis by x_pole
    let r_x = Matrix3::rotation_x(y_p);  // Rotation about X-axis by y_pole
    
    Ok(r_x * r_y)
}

/// Compute Greenwich Mean Sidereal Time (GMST) for given UT1 MJD
fn greenwich_mean_sidereal_time(ut1_mjd: f64) -> f64 {
    // GMST calculation based on IAU 2000 model
    let t_ut1 = (ut1_mjd - 51544.5) / 36525.0; // UT1 centuries since J2000.0
    
    // GMST at 0h UT1
    let gmst_0 = 24110.54841 + 8640184.812866 * t_ut1 + 0.093104 * t_ut1 * t_ut1 - 0.0000062 * t_ut1 * t_ut1 * t_ut1;
    
    // Add contribution from time of day
    let seconds_in_day = (ut1_mjd - ut1_mjd.floor()) * 86400.0;
    let gmst_seconds = gmst_0 + seconds_in_day * 1.00273790935;
    
    // Convert to radians and normalize
    let gmst_rad = gmst_seconds * std::f64::consts::PI / 43200.0; // 43200 = 86400/2 (seconds to radians)
    normalize_angle(gmst_rad)
}

/// Greenwich Apparent Sidereal Time (GAST) - includes nutation correction
/// For higher precision, GAST = GMST + equation of equinoxes
fn greenwich_apparent_sidereal_time(ut1_mjd: f64, tt_mjd: f64) -> f64 {
    let gmst = greenwich_mean_sidereal_time(ut1_mjd);
    let eq_eq = equation_of_equinoxes(tt_mjd);
    normalize_angle(gmst + eq_eq)
}

/// Equation of equinoxes (simplified)
fn equation_of_equinoxes(tt_mjd: f64) -> f64 {
    let t = (tt_mjd - 51544.5) / 36525.0; // TT centuries since J2000.0
    
    // Mean obliquity
    let epsilon_0 = 23.439291 - 0.0130042 * t;
    let eps_rad = epsilon_0 * std::f64::consts::PI / 180.0;
    
    // Simplified nutation in longitude (main term)
    let omega = longitude_ascending_node_simple(t);
    let dpsi = -17.2 * omega.sin() / 3600.0 * std::f64::consts::PI / 180.0; // arcsec to rad
    
    dpsi * eps_rad.cos()
}

/// Simplified longitude of ascending node for equation of equinoxes
fn longitude_ascending_node_simple(t: f64) -> f64 {
    let omega = 125.04 - 1934.136 * t;
    normalize_angle(omega * std::f64::consts::PI / 180.0)
}

/// Earth's rotation rate in rad/s
fn earth_rotation_rate() -> f64 {
    // Nominal Earth rotation rate
    7.2921159e-5 // rad/s
}

/// Normalize angle to [0, 2π]  
fn normalize_angle(angle: f64) -> f64 {
    let two_pi = 2.0 * std::f64::consts::PI;
    angle - two_pi * (angle / two_pi).floor()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EopData;
    use chrono::{DateTime, Utc, TimeZone};

    #[test]
    fn test_gmst_calculation() {
        // Test GMST at J2000.0 epoch
        let j2000_mjd = 51544.5; // J2000.0 at 12:00 UT
        let gmst = greenwich_mean_sidereal_time(j2000_mjd);
        
        // GMST at J2000.0 12:00 UT should be approximately 18h 41m 50.54841s
        let expected_hours = 18.0 + 41.0/60.0 + 50.54841/3600.0;
        let expected_rad = expected_hours * std::f64::consts::PI / 12.0;
        
        let diff = (gmst - expected_rad).abs();
        assert!(diff < 0.01, "GMST calculation error: expected {:.6}, got {:.6}, diff {:.6}", expected_rad, gmst, diff);
    }

    #[test]
    fn test_polar_motion_matrix() {
        // Test with zero polar motion (should be identity)
        let pm = polar_motion_matrix(0.0, 0.0).unwrap();
        let identity = Matrix3::identity();
        
        for i in 0..3 {
            for j in 0..3 {
                assert!((pm.data[i][j] - identity.data[i][j]).abs() < 1e-10, 
                       "Zero polar motion should give identity matrix");
            }
        }
        
        // Test determinant (should be 1 for rotation matrix)
        let det = pm.determinant();
        assert!((det - 1.0).abs() < 1e-10, "Polar motion matrix determinant should be 1");
    }

    #[test]
    fn test_earth_rotation_matrix() {
        let ut1_mjd = 51544.5; // J2000.0
        let rot = earth_rotation_matrix(ut1_mjd).unwrap();
        
        // Should be a rotation matrix (determinant = 1)
        let det = rot.determinant();
        assert!((det - 1.0).abs() < 1e-10, "Earth rotation matrix determinant should be 1");
        
        // Test that it's orthogonal: R * R^T = I
        let rt = rot.transpose();
        let product = rot * rt;
        let identity = Matrix3::identity();
        
        for i in 0..3 {
            for j in 0..3 {
                assert!((product.data[i][j] - identity.data[i][j]).abs() < 1e-10, 
                       "Rotation matrix should be orthogonal");
            }
        }
    }

    #[test]
    fn test_gcrs_to_itrs_transform() {
        // Create test context
        let time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).single().unwrap();
        let eop = EopData {
            mjd: 51544,
            x_pole: 0.1,   // Small polar motion
            y_pole: 0.2,
            ut1_utc: 0.0,
            lod: 0.0,
            dx_cip: None,
            dy_cip: None,
            dpsi: None,
            deps: None,
        };
        let ctx = CoordinateContext::new(time, eop).unwrap();
        
        // Test state vector
        let state = StateVector {
            position: [7000.0, 0.0, 0.0], // km 
            velocity: [0.0, 7.5, 0.0],    // km/s
        };
        
        let result = transform(&state, &ctx).unwrap();
        
        // Position magnitude should be preserved (roughly)
        let orig_pos_mag = (state.position[0].powi(2) + state.position[1].powi(2) + state.position[2].powi(2)).sqrt();
        let new_pos_mag = (result.position[0].powi(2) + result.position[1].powi(2) + result.position[2].powi(2)).sqrt();
        
        assert!((orig_pos_mag - new_pos_mag).abs() < 1e-3, "Position magnitude should be approximately preserved");
        
        // Velocity should change due to Earth rotation
        let orig_vel_mag = (state.velocity[0].powi(2) + state.velocity[1].powi(2) + state.velocity[2].powi(2)).sqrt();
        let new_vel_mag = (result.velocity[0].powi(2) + result.velocity[1].powi(2) + result.velocity[2].powi(2)).sqrt();
        
        // Velocity magnitude will change due to rotation effects, but should be reasonable
        assert!(new_vel_mag > 0.0, "Transformed velocity should be non-zero");
        assert!(new_vel_mag < 20.0, "Transformed velocity should be reasonable");
    }

    #[test]
    fn test_equation_of_equinoxes() {
        let tt_mjd = 51544.5; // J2000.0
        let eq_eq = equation_of_equinoxes(tt_mjd);
        
        // Should be small (arcsecond level)  
        assert!(eq_eq.abs() < 0.01, "Equation of equinoxes should be small");
    }

    #[test]
    fn test_earth_rotation_rate() {
        let omega = earth_rotation_rate();
        
        // Should be approximately 7.29e-5 rad/s
        assert!((omega - 7.2921159e-5).abs() < 1e-10, "Earth rotation rate incorrect");
        
        // Period should be approximately 1 sidereal day
        let period = 2.0 * std::f64::consts::PI / omega;
        let sidereal_day = 86164.0905; // seconds
        assert!((period - sidereal_day).abs() < 1.0, "Sidereal day calculation error");
    }
}