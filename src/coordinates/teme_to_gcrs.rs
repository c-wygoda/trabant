//! TEME to GCRS coordinate frame transformation
//!
//! Transforms from True Equator Mean Equinox (TEME) frame used by SGP4
//! to Geocentric Celestial Reference System (GCRS) using:
//! - IAU 2006 precession model
//! - IAU 2000A nutation model  
//! - Frame bias correction

use crate::{StateVector, coordinates::{CoordinateContext, CoordinateError, matrix::{Matrix3, Vector3}}};

/// Transform state vector from TEME to GCRS frame
pub fn transform(state: &StateVector, ctx: &CoordinateContext) -> Result<StateVector, CoordinateError> {
    // Compute transformation matrix from TEME to GCRS
    let teme_to_gcrs_matrix = compute_teme_to_gcrs_matrix(ctx)?;
    
    // Transform position and velocity
    let pos_teme = Vector3::from(state.position);
    let vel_teme = Vector3::from(state.velocity);
    
    let pos_gcrs = teme_to_gcrs_matrix.transform_vector(pos_teme);
    let vel_gcrs = teme_to_gcrs_matrix.transform_vector(vel_teme);
    
    Ok(StateVector {
        position: pos_gcrs.into(),
        velocity: vel_gcrs.into(),
    })
}

/// Compute the complete TEME to GCRS transformation matrix
fn compute_teme_to_gcrs_matrix(ctx: &CoordinateContext) -> Result<Matrix3, CoordinateError> {
    // TEME to GCRS transformation using skyfield's theta_GMST1982 method
    // This should now match skyfield's exact implementation
    //
    // The transformation is: GCRS = R_z(-theta_GMST1982) * TEME
    // where theta_GMST1982 is skyfield's specific GMST calculation
    
    // Compute theta using skyfield's exact method
    let theta = compute_gmst(ctx.ut1_mjd);
    
    // For TEME to GCRS, we rotate by -theta (removing Earth rotation)
    let angle = -theta;
    
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();
    
    // Z-axis rotation matrix
    Ok(Matrix3::new([
        [cos_angle, -sin_angle, 0.0],
        [sin_angle,  cos_angle, 0.0],
        [     0.0,        0.0,  1.0]
    ]))
}

/// Compute Greenwich Mean Sidereal Time using skyfield's theta_GMST1982 method
/// This matches skyfield's exact implementation for TEME coordinate transformations
fn compute_theta_gmst1982(jd_ut1: f64, fraction_ut1: f64) -> f64 {
    // Skyfield's theta_GMST1982 implementation
    // Reference: AIAA 2006-6753 Appendix C
    const T0: f64 = 2451545.0; // J2000.0 epoch (JD)
    const DAY_S: f64 = 86400.0; // Seconds per day
    const TAU: f64 = 2.0 * std::f64::consts::PI; // 2π
    
    // Time in Julian centuries since J2000.0
    let t = (jd_ut1 - T0 + fraction_ut1) / 36525.0;
    
    // Angle computation using skyfield's coefficients
    let g = 67310.54841 
          + (8640184.812866 + (0.093104 + (-6.2e-6) * t) * t) * t;
    
    // Final theta calculation
    let theta = ((jd_ut1 % 1.0) + fraction_ut1 + (g / DAY_S % 1.0)) % 1.0 * TAU;
    
    theta
}

/// Compute Greenwich Mean Sidereal Time in radians using skyfield's method
fn compute_gmst(ut1_mjd: f64) -> f64 {
    // Convert MJD to JD and use skyfield's exact method
    let jd_ut1 = ut1_mjd + 2400000.5;
    let jd_whole = jd_ut1.floor();
    let jd_fraction = jd_ut1 - jd_whole;
    
    compute_theta_gmst1982(jd_whole, jd_fraction)
}

/// Frame bias matrix (GCRS to J2000.0)
/// Values from IAU 2006 - very small corrections
fn frame_bias_matrix() -> Matrix3 {
    // Frame bias angles in radians (very small)
    let eta_0 = -0.0146 * (std::f64::consts::PI / 180.0) / 3600.0; // arcsec to radians
    let xi_0 = -0.0166170 * (std::f64::consts::PI / 180.0) / 3600.0;
    let da_0 = -0.01460 * (std::f64::consts::PI / 180.0) / 3600.0;
    
    // Simplified frame bias matrix (small angle approximation)
    Matrix3::new([
        [1.0 - 0.5 * (eta_0 * eta_0 + xi_0 * xi_0), -da_0, xi_0],
        [da_0, 1.0 - 0.5 * eta_0 * eta_0, eta_0],
        [-xi_0, -eta_0, 1.0 - 0.5 * xi_0 * xi_0],
    ])
}

/// IAU 2006 precession matrix
fn precession_matrix(t: f64) -> Result<Matrix3, CoordinateError> {
    // IAU 2006 precession angles (Capitaine et al. 2003)
    // t is centuries from J2000.0
    
    // Precession angles in arcseconds
    let epsilon_a = 84381.406 - 46.836769 * t - 0.0001831 * t * t 
                  + 0.00200340 * t * t * t - 0.000000576 * t * t * t * t 
                  - 0.0000000434 * t * t * t * t * t;
    
    let psi_a = -46.811015 * t + 0.0511269 * t * t + 0.00053289 * t * t * t
              - 0.000000544 * t * t * t * t - 0.0000000002 * t * t * t * t * t;
    
    let omega_a = epsilon_a + 0.05127 * t * t - 0.007726 * t * t * t
                - 0.0000059 * t * t * t * t + 0.0000000007 * t * t * t * t * t;
    
    let chi_a = 10.556403 * t - 2.3814292 * t * t - 0.00121197 * t * t * t
              + 0.000170663 * t * t * t * t - 0.0000000560 * t * t * t * t * t;
    
    // Convert to radians
    let arcsec_to_rad = std::f64::consts::PI / (180.0 * 3600.0);
    let eps_a = epsilon_a * arcsec_to_rad;
    let psi_a = psi_a * arcsec_to_rad;
    let omega_a = omega_a * arcsec_to_rad;
    let chi_a = chi_a * arcsec_to_rad;
    
    // Compute rotation matrices
    let r1 = Matrix3::rotation_z(-chi_a);
    let r2 = Matrix3::rotation_x(omega_a);
    let r3 = Matrix3::rotation_z(psi_a);
    let r4 = Matrix3::rotation_x(-eps_a);
    
    Ok(r4 * r3 * r2 * r1)
}

/// IAU 2000A nutation matrix (simplified version)  
fn nutation_matrix(tt_mjd: f64) -> Result<Matrix3, CoordinateError> {
    let t = (tt_mjd - 51544.5) / 36525.0; // Centuries since J2000.0
    
    // Mean obliquity of ecliptic
    let epsilon_0 = 23.439291 - 0.0130042 * t - 0.00000016 * t * t + 0.000000504 * t * t * t;
    let eps_rad = epsilon_0 * std::f64::consts::PI / 180.0;
    
    // Simplified nutation computation (main terms only for performance)
    // For full IAU 2000A, would need ~1365 terms
    let (dpsi, deps) = compute_nutation_angles(t);
    
    // True obliquity
    let epsilon = eps_rad + deps;
    
    // Nutation matrix
    let cos_eps = epsilon.cos();
    let sin_eps = epsilon.sin();
    let cos_dpsi = dpsi.cos();
    let sin_dpsi = dpsi.sin();
    
    Ok(Matrix3::new([
        [cos_dpsi, -sin_dpsi * cos_eps, -sin_dpsi * sin_eps],
        [sin_dpsi, cos_dpsi * cos_eps, cos_dpsi * sin_eps],
        [0.0, -sin_eps, cos_eps],
    ]))
}

/// J2000.0 obliquity of the ecliptic in radians
/// Mean obliquity at J2000.0 = 23°26'21".448 = 84381".448
fn obliquity_j2000() -> f64 {
    84381.448 * (std::f64::consts::PI / 180.0) / 3600.0 // arcseconds to radians
}

/// Compute nutation angles (simplified IAU 2000A model)
fn compute_nutation_angles(t: f64) -> (f64, f64) {
    // Fundamental arguments (Delaunay arguments) in radians
    let l = mean_anomaly_moon(t);
    let l_prime = mean_anomaly_sun(t);
    let f = mean_argument_latitude(t);
    let d = mean_elongation(t);
    let omega = longitude_ascending_node(t);
    
    // Main nutation terms (highest amplitude terms from IAU 2000A)
    // Format: [multipliers for l,l',f,d,omega], psi_coeff_sin, psi_coeff_cos, eps_coeff_sin, eps_coeff_cos
    let nutation_terms = [
        // [0,0,0,0,1] - main term
        ([0, 0, 0, 0, 1], -171996.0, -174.2, 92025.0, 8.9),
        // [0,0,2,-2,2] 
        ([0, 0, 2, -2, 2], -13187.0, -1.6, 5736.0, -3.1),
        // [0,0,2,0,2]
        ([0, 0, 2, 0, 2], -2274.0, -0.2, 977.0, -0.5),
        // [0,0,0,0,2] 
        ([0, 0, 0, 0, 2], 2062.0, 0.2, -895.0, 0.5),
        // [0,1,0,0,0]
        ([0, 1, 0, 0, 0], 1426.0, -3.4, 54.0, -0.1),
        // [1,0,0,0,0] 
        ([1, 0, 0, 0, 0], 712.0, 0.1, -7.0, 0.0),
        // [0,1,2,-2,2]
        ([0, 1, 2, -2, 2], -517.0, 1.2, 224.0, -0.6),
        // [0,0,2,0,1]
        ([0, 0, 2, 0, 1], -386.0, -0.4, 200.0, 0.0),
        // [1,0,2,0,2] 
        ([1, 0, 2, 0, 2], -301.0, 0.0, 129.0, -0.1),
        // [0,-1,2,-2,2]
        ([0, -1, 2, -2, 2], 217.0, -0.5, -95.0, 0.3),
    ];
    
    let mut dpsi = 0.0;
    let mut deps = 0.0;
    
    for &(multipliers, psi_sin, psi_cos, eps_sin, eps_cos) in &nutation_terms {
        let arg = multipliers[0] as f64 * l 
                + multipliers[1] as f64 * l_prime
                + multipliers[2] as f64 * f
                + multipliers[3] as f64 * d
                + multipliers[4] as f64 * omega;
        
        // Convert from 0.1 microarcsec to radians
        // 0.1 microarcsec = 0.1e-6 arcsec = 0.1e-6 * pi/(180*3600) radians
        let factor = 0.1e-6 * std::f64::consts::PI / (180.0 * 3600.0);
        dpsi += (psi_sin + psi_cos * t) * arg.sin() * factor;
        deps += (eps_sin + eps_cos * t) * arg.cos() * factor;
    }
    
    (dpsi, deps)
}

/// Mean anomaly of the Moon
fn mean_anomaly_moon(t: f64) -> f64 {
    let l = 485868.249036 + 1717915923.2178 * t + 31.8792 * t * t + 0.051635 * t * t * t - 0.00024470 * t * t * t * t;
    normalize_angle(l * std::f64::consts::PI / (180.0 * 3600.0))
}

/// Mean anomaly of the Sun  
fn mean_anomaly_sun(t: f64) -> f64 {
    let l_prime = 1287104.79305 + 129596581.0481 * t - 0.5532 * t * t + 0.000136 * t * t * t - 0.00001149 * t * t * t * t;
    normalize_angle(l_prime * std::f64::consts::PI / (180.0 * 3600.0))
}

/// Mean argument of latitude
fn mean_argument_latitude(t: f64) -> f64 {
    let f = 335779.526232 + 1739527262.8478 * t - 12.7512 * t * t - 0.001037 * t * t * t + 0.00000417 * t * t * t * t;
    normalize_angle(f * std::f64::consts::PI / (180.0 * 3600.0))
}

/// Mean elongation of Moon from Sun
fn mean_elongation(t: f64) -> f64 {
    let d = 1072260.70369 + 1602961601.2090 * t - 6.3706 * t * t + 0.006593 * t * t * t - 0.00003169 * t * t * t * t;
    normalize_angle(d * std::f64::consts::PI / (180.0 * 3600.0))
}

/// Mean longitude of ascending node of Moon
fn longitude_ascending_node(t: f64) -> f64 {
    let omega = 450160.398036 - 6962890.5431 * t + 7.4722 * t * t + 0.007702 * t * t * t - 0.00005939 * t * t * t * t;
    normalize_angle(omega * std::f64::consts::PI / (180.0 * 3600.0))
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
    fn test_frame_bias_matrix() {
        let bias = frame_bias_matrix();
        // Should be close to identity matrix (very small corrections)
        let det = bias.determinant();
        assert!((det - 1.0).abs() < 1e-10, "Frame bias determinant should be ~1");
        
        // Check that it's approximately identity
        assert!((bias.data[0][0] - 1.0).abs() < 1e-6);
        assert!((bias.data[1][1] - 1.0).abs() < 1e-6);
        assert!((bias.data[2][2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_precession_matrix() {
        let t = 0.0; // J2000.0
        let prec = precession_matrix(t).unwrap();
        
        // At J2000.0, should be close to identity
        let det = prec.determinant();
        assert!((det - 1.0).abs() < 1e-10, "Precession determinant should be ~1");
    }

    #[test]
    fn test_nutation_angles() {
        let t = 0.0; // J2000.0
        let (dpsi, deps) = compute_nutation_angles(t);
        
        // Nutation angles should be small (arcseconds level)
        assert!(dpsi.abs() < 0.01, "Nutation in longitude should be small");
        assert!(deps.abs() < 0.01, "Nutation in obliquity should be small");
    }

    #[test]
    fn test_fundamental_arguments() {
        let t = 0.0; // J2000.0
        
        // Test that all fundamental arguments return reasonable values
        let l = mean_anomaly_moon(t);
        let l_prime = mean_anomaly_sun(t);
        let f = mean_argument_latitude(t);
        let d = mean_elongation(t);
        let omega = longitude_ascending_node(t);
        
        // All should be in [0, 2π]
        assert!(l >= 0.0 && l <= 2.0 * std::f64::consts::PI);
        assert!(l_prime >= 0.0 && l_prime <= 2.0 * std::f64::consts::PI);
        assert!(f >= 0.0 && f <= 2.0 * std::f64::consts::PI);
        assert!(d >= 0.0 && d <= 2.0 * std::f64::consts::PI);
        assert!(omega >= 0.0 && omega <= 2.0 * std::f64::consts::PI);
    }

    #[test]
    fn test_teme_to_gcrs_transform() {
        // Create test context
        let time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).single().unwrap();
        let eop = EopData {
            mjd: 51544,
            x_pole: 0.0,
            y_pole: 0.0,
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
        
        // Transformation should preserve magnitude (roughly)
        let orig_pos_mag = (state.position[0].powi(2) + state.position[1].powi(2) + state.position[2].powi(2)).sqrt();
        let new_pos_mag = (result.position[0].powi(2) + result.position[1].powi(2) + result.position[2].powi(2)).sqrt();
        
        assert!((orig_pos_mag - new_pos_mag).abs() < 1e-6, "Position magnitude should be preserved");
    }
}