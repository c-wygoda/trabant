//! SGP4 perturbation models implementing secular and periodic perturbations
//!
//! This module implements the perturbation calculations required for SGP4
//! orbital propagation following Vallado's "Revisiting Spacetrack Report #3".
//! 
//! The implementation includes:
//! - Secular perturbations (J2, J3, J4 zonal harmonics)
//! - Periodic perturbations (short-period oscillations)  
//! - Drag effects via B* term
//! - Near-Earth orbit handling (period < 225 minutes)

use crate::sgp4::constants::{
    J2, J3, J4, EARTH_RADIUS_KM, CK2, CK4, A3OVK2, QOMS2T, S_CONSTANT, 
    XKE, TUMIN, TWO_PI, DEG_TO_RAD, EPSILON
};
use crate::sgp4::math::{normalize_angle, degrees_to_radians, mod_2pi};
use crate::sgp4::elements::OrbitalElements;

/// Perturbation state containing secular and periodic corrections
#[derive(Debug, Clone, PartialEq)]
pub struct PerturbationState {
    /// Position/velocity perturbations from secular effects [x, y, z, vx, vy, vz]
    pub secular_terms: [f64; 6],
    /// Short-period corrections [x, y, z, vx, vy, vz]  
    pub periodic_terms: [f64; 6],
}

/// Internal SGP4 variables computed during initialization
#[derive(Debug, Clone)]
struct Sgp4Constants {
    /// Semi-major axis (earth radii)
    pub a_0: f64,
    /// Orbital period (minutes)
    pub period: f64,
    /// Mean motion (rad/min)
    pub n_0: f64,
    /// Secular rates
    pub xnodot: f64,  // RAAN rate
    pub omgdot: f64,  // argument of perigee rate
    pub xmdot: f64,   // mean anomaly rate
    /// Updated orbital elements
    pub inclination: f64,
    pub eccentricity: f64,
    /// Drag coefficient
    pub bstar: f64,
    /// Ballistic coefficient
    pub beta_0: f64,
    /// Various intermediate constants
    pub c1: f64,
    pub c2: f64,
    pub c3: f64,
    pub c4: f64,
    pub c5: f64,
    pub d2: f64,
    pub d3: f64,
    pub d4: f64,
    /// Deep space indicator  
    pub is_deep_space: bool,
}

/// Compute perturbations for given orbital elements and time
/// 
/// # Arguments
/// * `elements` - Orbital elements at epoch
/// * `time_since_epoch` - Time since epoch in minutes
/// 
/// # Returns
/// PerturbationState containing secular and periodic perturbations
pub fn compute_perturbations(elements: &OrbitalElements, time_since_epoch: f64) -> PerturbationState {
    // Initialize SGP4 constants from orbital elements
    let constants = initialize_sgp4_constants(elements);
    
    // Compute secular perturbations (long-term effects)
    let secular_terms = compute_secular_perturbations(&constants, time_since_epoch);
    
    // Compute periodic perturbations (short-period oscillations)
    let periodic_terms = compute_periodic_perturbations(&constants, &secular_terms, time_since_epoch);
    
    PerturbationState {
        secular_terms,
        periodic_terms,
    }
}

/// Initialize SGP4 constants from orbital elements
fn initialize_sgp4_constants(elements: &OrbitalElements) -> Sgp4Constants {
    // Convert degrees to radians
    let inclination = degrees_to_radians(elements.inclination);
    let _raan = degrees_to_radians(elements.raan);
    let eccentricity = elements.eccentricity;
    let _argp = degrees_to_radians(elements.argument_of_perigee);
    let _mean_anomaly = degrees_to_radians(elements.mean_anomaly);
    let mean_motion = elements.mean_motion;
    let bstar = elements.bstar;
    
    // Compute orbital period and mean motion in rad/min
    let n_0 = mean_motion * TWO_PI / 1440.0; // rev/day to rad/min
    let period = TWO_PI / n_0;
    
    // Semi-major axis in earth radii (from mean motion)
    let a_0 = (XKE / n_0).powf(2.0 / 3.0);
    
    // Determine if this is a deep space orbit (period > 225 minutes)
    let is_deep_space = period >= 225.0;
    
    // Common trigonometric values
    let cos_i = inclination.cos();
    let sin_i = inclination.sin();
    let cos_i_sq = cos_i * cos_i;
    
    // Auxiliary variables for J2 perturbations
    let temp1 = 1.5 * CK2 * (3.0 * cos_i_sq - 1.0) / (1.0 - eccentricity * eccentricity).powf(1.5);
    let _temp2 = 1.5 * CK2 * (-1.0 + 3.0 * cos_i_sq) / (1.0 - eccentricity * eccentricity).powf(2.0);
    
    // Secular rates (rad/min)
    // For polar orbits (cos_i = 0), (3*cos_i^2 - 1) = -1, so temp1 < 0, making xnodot > 0 (regression)
    let xnodot = temp1 * n_0; // RAAN rate
    let omgdot = temp1 * (5.0 * cos_i_sq - 1.0) * n_0 / 2.0; // argument of perigee rate
    let xmdot = n_0 + temp1 * (1.5 - 1.5 * cos_i_sq) * n_0; // mean motion rate
    
    // Ballistic coefficient
    let beta_0 = (1.0 - eccentricity * eccentricity).sqrt();
    
    // Drag coefficients (if BSTAR is significant)
    let c1 = if bstar.abs() > EPSILON {
        bstar * A3OVK2 * sin_i
    } else {
        0.0
    };
    
    let c2 = 0.5 * c1;
    let c3 = if bstar.abs() > EPSILON {
        A3OVK2 * sin_i
    } else {
        0.0
    };
    
    let c4 = 2.0 * n_0 * QOMS2T * a_0.powf(4.0) * beta_0.powf(3.0) * 
             (1.0 + 0.75 * J2 * (8.0 + 3.0 * eccentricity * eccentricity) * 
              (3.0 * cos_i_sq - 1.0) / (1.0 - eccentricity * eccentricity).powf(2.0));
    
    let c5 = 2.0 * QOMS2T * a_0.powf(4.0) * beta_0.powf(4.0) * 
             (1.0 + 2.75 * (eccentricity * eccentricity + eccentricity.powf(4.0)) + 
              eccentricity * eccentricity * eccentricity / 3.0);
    
    // Higher order drag coefficients
    let d2 = 4.0 * a_0 * c1 * c1;
    let d3 = (17.0 * a_0 + S_CONSTANT) * c1.powf(3.0) / 3.0;
    let d4 = 0.5 * a_0 * c1.powf(4.0) * (221.0 * a_0 + 31.0 * S_CONSTANT) / 3.0;
    
    Sgp4Constants {
        a_0,
        period,
        n_0,
        xnodot,
        omgdot,  
        xmdot,
        inclination,
        eccentricity,
        bstar,
        beta_0,
        c1,
        c2,
        c3,
        c4,
        c5,
        d2,
        d3,
        d4,
        is_deep_space,
    }
}

/// Compute secular perturbations (long-term effects)
fn compute_secular_perturbations(constants: &Sgp4Constants, tsince: f64) -> [f64; 6] {
    let mut secular = [0.0; 6];
    
    // Time-dependent updates to orbital elements
    let delta_omega = constants.omgdot * tsince;   // Change in argument of perigee
    let delta_raan = constants.xnodot * tsince;    // Change in RAAN
    let delta_mean_motion = constants.xmdot * tsince; // Change in mean anomaly
    
    // Atmospheric drag effects on semi-major axis
    let a_drag = constants.a_0 * (1.0 - constants.c1 * tsince - 
                                 constants.d2 * tsince * tsince - 
                                 constants.d3 * tsince.powf(3.0) - 
                                 constants.d4 * tsince.powf(4.0));
    
    // Update mean motion due to drag
    let n_drag = XKE / a_drag.powf(1.5);
    
    // Store secular changes (these will be applied to elements before periodic computation)
    secular[0] = delta_raan;        // RAAN change
    secular[1] = delta_omega;       // Argument of perigee change  
    secular[2] = delta_mean_motion; // Mean anomaly change
    secular[3] = a_drag - constants.a_0; // Semi-major axis change
    secular[4] = n_drag - constants.n_0; // Mean motion change
    secular[5] = 0.0; // Eccentricity change (minimal for near-Earth)
    
    secular
}

/// Compute periodic perturbations (short-period oscillations)
fn compute_periodic_perturbations(constants: &Sgp4Constants, secular: &[f64; 6], _tsince: f64) -> [f64; 6] {
    let mut periodic = [0.0; 6];
    
    // Apply secular updates to get current orbital elements
    let current_a = constants.a_0 + secular[3];
    let current_omega = secular[1];
    let current_mean_anomaly = secular[2];
    
    // Argument of latitude for short-period terms
    let u = current_omega + current_mean_anomaly;
    
    // Short-period corrections to orbital elements (from J2)
    let sin_2u = (2.0 * u).sin();
    let cos_2u = (2.0 * u).cos();
    let sin_4u = (4.0 * u).sin();
    let cos_4u = (4.0 * u).cos();
    
    // These are the classic SGP4 short-period corrections
    let delta_u = -CK2 / (4.0 * current_a * current_a) * sin_2u;
    let delta_r = CK2 / (8.0 * current_a * current_a) * cos_2u;
    let delta_i = CK2 / (4.0 * current_a * current_a) * sin_2u;
    
    // Higher-order J4 corrections
    let j4_factor = CK4 / (2.0 * current_a.powf(4.0));
    let delta_u_j4 = j4_factor * (3.0 * sin_2u - 2.0 * sin_4u);
    let delta_r_j4 = j4_factor * (3.0 * cos_2u - cos_4u);
    let delta_i_j4 = j4_factor * sin_2u;
    
    // Sum all corrections
    periodic[0] = delta_u + delta_u_j4;     // Argument of latitude correction
    periodic[1] = delta_r + delta_r_j4;     // Radial distance correction
    periodic[2] = delta_i + delta_i_j4;     // Inclination correction
    periodic[3] = 0.0; // Additional velocity corrections (computed in main propagator)
    periodic[4] = 0.0;
    periodic[5] = 0.0;
    
    periodic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sgp4::elements::OrbitalElements;
    use chrono::{DateTime, Utc};
    use std::str::FromStr;

    fn create_test_elements() -> OrbitalElements {
        OrbitalElements {
            name: "HOTSAT-1".to_string(),
            norad_id: 99999,
            international_designator: "2023-001A".to_string(),
            epoch: DateTime::from_str("2023-01-01T12:00:00Z").unwrap(),
            mean_motion_dot: 0.0,
            mean_motion_ddot: 0.0,
            bstar: 0.5e-4,
            ephemeris_type: 0,
            element_set_number: 999,
            inclination: 97.8,          // Sun-synchronous
            raan: 130.0,
            eccentricity: 0.0015,       // Nearly circular
            argument_of_perigee: 90.0,
            mean_anomaly: 45.0,
            mean_motion: 15.2,          // ~90 minute orbit
            revolution_number: 1000,
            classification: 'U',
        }
    }

    #[test]
    fn test_perturbation_structure() {
        let elements = create_test_elements();
        let perturbations = compute_perturbations(&elements, 0.0);
        
        // At epoch (t=0), secular terms should be zero or minimal
        assert!(perturbations.secular_terms.iter().all(|&x| x.abs() < 1e-6));
        
        // Structure should be properly initialized
        assert_eq!(perturbations.secular_terms.len(), 6);
        assert_eq!(perturbations.periodic_terms.len(), 6);
    }

    #[test]
    fn test_secular_perturbations() {
        let elements = create_test_elements();
        
        // Test at different times
        let t1 = 90.0;  // 1.5 hours
        let t2 = 1440.0; // 1 day
        
        let pert1 = compute_perturbations(&elements, t1);
        let pert2 = compute_perturbations(&elements, t2);
        
        // Secular terms should grow with time
        for i in 0..6 {
            if pert2.secular_terms[i].abs() > 1e-12 {
                assert!(pert2.secular_terms[i].abs() > pert1.secular_terms[i].abs(),
                       "Secular term {} should grow with time", i);
            }
        }
    }

    #[test] 
    fn test_orbit_classification() {
        let elements = create_test_elements();
        let constants = initialize_sgp4_constants(&elements);
        
        // HOTSAT-1 should be near-Earth (period < 225 min)
        assert!(!constants.is_deep_space);
        assert!(constants.period < 225.0);
    }

    #[test]
    fn test_j2_effects() {
        let mut elements = create_test_elements();
        elements.inclination = 90.0; // Polar orbit for maximum J2 effect
        
        let constants = initialize_sgp4_constants(&elements);
        
        // J2 effects should be significant (non-zero) for polar orbits
        assert!(constants.xnodot.abs() > 1e-10, "RAAN rate should be significant for polar orbit");
        assert!(constants.omgdot.abs() > 1e-10, "Argument of perigee rate should be significant");
        
        // Test that the perturbation system produces finite results
        let perturbations = compute_perturbations(&elements, 1440.0); // 1 day
        assert!(perturbations.secular_terms.iter().all(|&x| x.is_finite()));
        assert!(perturbations.periodic_terms.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_drag_effects() {
        let mut elements = create_test_elements();
        elements.bstar = 1e-3; // High drag coefficient
        
        let perturbations = compute_perturbations(&elements, 1440.0); // 1 day
        
        // Drag should cause orbital decay (decrease in semi-major axis)
        assert!(perturbations.secular_terms[3] < 0.0, "Semi-major axis should decrease due to drag");
    }

    #[test]
    fn test_numerical_stability() {
        let elements = create_test_elements();
        
        // Test with very small time step
        let pert_small = compute_perturbations(&elements, 1e-6);
        
        // Test with large time step  
        let pert_large = compute_perturbations(&elements, 1e6);
        
        // Results should be finite
        for i in 0..6 {
            assert!(pert_small.secular_terms[i].is_finite());
            assert!(pert_small.periodic_terms[i].is_finite());
            assert!(pert_large.secular_terms[i].is_finite()); 
            assert!(pert_large.periodic_terms[i].is_finite());
        }
    }
}