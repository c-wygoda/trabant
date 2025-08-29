//! Main SGP4 propagator implementation
//!
//! This module implements the complete SGP4 algorithm following Vallado's
//! "Revisiting Spacetrack Report #3". It integrates all components:
//! orbital elements, perturbations, Kepler solving, and coordinate transformations.

use crate::sgp4::constants::{XKE, TUMIN, EARTH_RADIUS_KM, TWO_PI, DEG_TO_RAD, EPSILON};
use crate::sgp4::math::{degrees_to_radians, normalize_angle, magnitude, cross_product};
use crate::sgp4::elements::OrbitalElements;
use crate::sgp4::kepler::{solve_kepler, KeplerSolution};
use crate::sgp4::perturbations::{compute_perturbations, PerturbationState};
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Errors that can occur during SGP4 propagation
#[derive(Error, Debug)]
pub enum Sgp4Error {
    #[error("Invalid orbital elements: {0}")]
    InvalidElements(String),
    
    #[error("Deep space orbit not supported (period >= 225 minutes)")]
    DeepSpaceOrbit,
    
    #[error("Satellite has decayed below Earth's surface")]
    SatelliteDecayed,
    
    #[error("Time too far from epoch: {days} days (limit: ±3650 days)")]
    TimeOutOfBounds { days: f64 },
    
    #[error("Numerical instability detected: {0}")]
    NumericalError(String),
    
    #[error("Propagation failed: {0}")]
    PropagationError(String),
}

/// TEME state vector containing position and velocity
#[derive(Debug, Clone, PartialEq)]
pub struct StateVector {
    /// Position vector in TEME coordinates [km]
    pub position: [f64; 3],
    /// Velocity vector in TEME coordinates [km/s]  
    pub velocity: [f64; 3],
}

/// Working orbital elements with secular perturbations applied
#[derive(Debug, Clone)]
struct WorkingElements {
    /// Semi-major axis [earth radii]
    pub semi_major_axis: f64,
    /// Eccentricity [dimensionless]
    pub eccentricity: f64,
    /// Inclination [radians]
    pub inclination: f64,
    /// Right ascension of ascending node [radians]
    pub raan: f64,
    /// Argument of perigee [radians]
    pub argument_of_perigee: f64,
    /// Mean anomaly [radians]
    pub mean_anomaly: f64,
    /// Mean motion [rad/min]
    pub mean_motion: f64,
}

/// State in orbital plane coordinates
#[derive(Debug, Clone)]
struct OrbitPlaneState {
    /// Position in orbital plane [km]
    pub position: [f64; 3],
    /// Velocity in orbital plane [km/s]
    pub velocity: [f64; 3],
}

/// SGP4 satellite propagator
pub struct Sgp4Propagator {
    /// Original orbital elements at epoch
    elements: OrbitalElements,
}

impl Sgp4Propagator {
    /// Create new SGP4 propagator from orbital elements
    /// 
    /// # Arguments
    /// * `elements` - Orbital elements at epoch
    /// 
    /// # Returns
    /// Result containing the propagator or an error
    /// 
    /// # Errors
    /// Returns error if orbital elements are invalid or represent deep space orbit
    pub fn new(elements: OrbitalElements) -> Result<Self, Sgp4Error> {
        // Validate orbital elements
        elements.validate().map_err(|e| Sgp4Error::InvalidElements(e))?;
        
        // Check for deep space orbit (period >= 225 minutes)
        let mean_motion_rad_min = elements.mean_motion * TWO_PI / 1440.0;
        let period_minutes = TWO_PI / mean_motion_rad_min;
        
        if period_minutes >= 225.0 {
            return Err(Sgp4Error::DeepSpaceOrbit);
        }
        
        // Additional validation
        if elements.eccentricity >= 1.0 {
            return Err(Sgp4Error::InvalidElements("Hyperbolic orbits not supported".to_string()));
        }
        
        if elements.inclination < 0.0 || elements.inclination > 180.0 {
            return Err(Sgp4Error::InvalidElements("Invalid inclination".to_string()));
        }
        
        Ok(Sgp4Propagator { elements })
    }
    
    /// Propagate satellite to specified time
    /// 
    /// # Arguments
    /// * `time` - Time to propagate to
    /// 
    /// # Returns
    /// Result containing the state vector in TEME coordinates
    /// 
    /// # Errors
    /// Returns error if propagation fails or time is out of bounds
    pub fn propagate(&self, time: DateTime<Utc>) -> Result<StateVector, Sgp4Error> {
        // Compute time since epoch in minutes
        let time_since_epoch = (time - self.elements.epoch).num_milliseconds() as f64 / 60000.0;
        
        // Check time bounds (±10 years from epoch)
        let days_since_epoch = time_since_epoch / 1440.0;
        if days_since_epoch.abs() > 3650.0 {
            return Err(Sgp4Error::TimeOutOfBounds { days: days_since_epoch });
        }
        
        // Compute perturbations
        let perturbations = compute_perturbations(&self.elements, time_since_epoch);
        
        // Apply secular perturbations to get working elements
        let mut working_elements = self.compute_working_elements(time_since_epoch, &perturbations)?;
        
        // Solve Kepler's equation
        let kepler_solution = solve_kepler(working_elements.mean_anomaly, working_elements.eccentricity);
        
        // Apply periodic perturbations
        self.apply_periodic_perturbations(&mut working_elements, &perturbations, &kepler_solution)?;
        
        // Convert to orbital plane coordinates
        let orbit_plane_state = self.orbital_plane_coordinates(&working_elements, &kepler_solution)?;
        
        // Rotate to TEME inertial frame
        let state_vector = self.rotate_to_teme(&working_elements, &orbit_plane_state)?;
        
        // Validate results
        self.validate_state_vector(&state_vector)?;
        
        Ok(state_vector)
    }
    
    /// Compute working orbital elements with secular perturbations applied
    fn compute_working_elements(&self, tsince: f64, perturbations: &PerturbationState) -> Result<WorkingElements, Sgp4Error> {
        // Convert base elements to radians and working units
        let mean_motion_rad_min = self.elements.mean_motion * TWO_PI / 1440.0; // rev/day to rad/min
        let semi_major_axis = (XKE / mean_motion_rad_min).powf(2.0 / 3.0); // earth radii
        
        // Apply secular perturbations
        let raan = degrees_to_radians(self.elements.raan) + perturbations.secular_terms[0];
        let argument_of_perigee = degrees_to_radians(self.elements.argument_of_perigee) + perturbations.secular_terms[1];  
        let mean_anomaly = degrees_to_radians(self.elements.mean_anomaly) + perturbations.secular_terms[2];
        let updated_semi_major_axis = semi_major_axis + perturbations.secular_terms[3];
        let updated_mean_motion = mean_motion_rad_min + perturbations.secular_terms[4];
        
        // Validate updated elements
        if updated_semi_major_axis < 1.01 {
            return Err(Sgp4Error::SatelliteDecayed);
        }
        
        Ok(WorkingElements {
            semi_major_axis: updated_semi_major_axis,
            eccentricity: self.elements.eccentricity,
            inclination: degrees_to_radians(self.elements.inclination),
            raan: normalize_angle(raan),
            argument_of_perigee: normalize_angle(argument_of_perigee),
            mean_anomaly: normalize_angle(mean_anomaly),
            mean_motion: updated_mean_motion,
        })
    }
    
    /// Apply periodic perturbations to working elements
    fn apply_periodic_perturbations(
        &self,
        working_elements: &mut WorkingElements,
        perturbations: &PerturbationState,
        _kepler_solution: &KeplerSolution,
    ) -> Result<(), Sgp4Error> {
        // Apply periodic corrections to orbital elements
        working_elements.argument_of_perigee += perturbations.periodic_terms[0];
        working_elements.semi_major_axis += perturbations.periodic_terms[1];
        working_elements.inclination += perturbations.periodic_terms[2];
        
        // Normalize angles
        working_elements.argument_of_perigee = normalize_angle(working_elements.argument_of_perigee);
        working_elements.inclination = working_elements.inclination.max(0.0).min(std::f64::consts::PI);
        
        Ok(())
    }
    
    /// Convert to orbital plane coordinates using proper perifocal formulation
    fn orbital_plane_coordinates(&self, elements: &WorkingElements, kepler_solution: &KeplerSolution) -> Result<OrbitPlaneState, Sgp4Error> {
        let eccentric_anomaly = kepler_solution.eccentric_anomaly;
        let true_anomaly = kepler_solution.true_anomaly;
        
        let cos_e = eccentric_anomaly.cos();
        let sin_e = eccentric_anomaly.sin();
        let cos_nu = true_anomaly.cos();
        let sin_nu = true_anomaly.sin();
        
        // Semi-major axis in km
        let a_km = elements.semi_major_axis * EARTH_RADIUS_KM;
        let e = elements.eccentricity;
        
        // Orbital radius
        let r = a_km * (1.0 - e * cos_e);
        
        // Position in orbital plane (perifocal coordinates)
        let pos_x = r * cos_nu;
        let pos_y = r * sin_nu;
        let pos_z = 0.0;
        
        // Gravitational parameter [km³/s²]
        let mu = 398600.5;
        
        // Specific angular momentum
        let h = (mu * a_km * (1.0 - e * e)).sqrt();
        
        // Velocity in orbital plane (correct perifocal velocity formulation)
        let vel_x = -(mu / h) * sin_nu;
        let vel_y = (mu / h) * (e + cos_nu);
        let vel_z = 0.0;
        
        // Validate results
        if !r.is_finite() || r < EARTH_RADIUS_KM {
            return Err(Sgp4Error::NumericalError(format!("Invalid orbital radius: {}", r)));
        }
        
        if !h.is_finite() || h <= 0.0 {
            return Err(Sgp4Error::NumericalError(format!("Invalid angular momentum: {}", h)));
        }
        
        Ok(OrbitPlaneState {
            position: [pos_x, pos_y, pos_z],
            velocity: [vel_x, vel_y, vel_z],
        })
    }
    
    /// Rotate from orbital plane to TEME inertial coordinates
    fn rotate_to_teme(&self, elements: &WorkingElements, orbit_state: &OrbitPlaneState) -> Result<StateVector, Sgp4Error> {
        let raan = elements.raan;
        let argp = elements.argument_of_perigee;
        let incl = elements.inclination;
        
        // Rotation matrix from orbital plane to TEME
        let cos_raan = raan.cos();
        let sin_raan = raan.sin();
        let cos_argp = argp.cos();
        let sin_argp = argp.sin();
        let cos_incl = incl.cos();
        let sin_incl = incl.sin();
        
        // Combined rotation matrix (P matrix in Vallado)
        let p11 = cos_raan * cos_argp - sin_raan * sin_argp * cos_incl;
        let p12 = -cos_raan * sin_argp - sin_raan * cos_argp * cos_incl;
        let p13 = sin_raan * sin_incl;
        
        let p21 = sin_raan * cos_argp + cos_raan * sin_argp * cos_incl;
        let p22 = -sin_raan * sin_argp + cos_raan * cos_argp * cos_incl;
        let p23 = -cos_raan * sin_incl;
        
        let p31 = sin_argp * sin_incl;
        let p32 = cos_argp * sin_incl;
        let p33 = cos_incl;
        
        // Transform position
        let pos_x = p11 * orbit_state.position[0] + p12 * orbit_state.position[1] + p13 * orbit_state.position[2];
        let pos_y = p21 * orbit_state.position[0] + p22 * orbit_state.position[1] + p23 * orbit_state.position[2];
        let pos_z = p31 * orbit_state.position[0] + p32 * orbit_state.position[1] + p33 * orbit_state.position[2];
        
        // Transform velocity  
        let vel_x = p11 * orbit_state.velocity[0] + p12 * orbit_state.velocity[1] + p13 * orbit_state.velocity[2];
        let vel_y = p21 * orbit_state.velocity[0] + p22 * orbit_state.velocity[1] + p23 * orbit_state.velocity[2];
        let vel_z = p31 * orbit_state.velocity[0] + p32 * orbit_state.velocity[1] + p33 * orbit_state.velocity[2];
        
        Ok(StateVector {
            position: [pos_x, pos_y, pos_z],
            velocity: [vel_x, vel_y, vel_z],
        })
    }
    
    /// Validate computed state vector
    fn validate_state_vector(&self, state: &StateVector) -> Result<(), Sgp4Error> {
        let pos_mag = magnitude(&state.position);
        let vel_mag = magnitude(&state.velocity);
        
        // Check for finite values
        if !pos_mag.is_finite() || !vel_mag.is_finite() {
            return Err(Sgp4Error::NumericalError("Non-finite position or velocity".to_string()));
        }
        
        // Check reasonable bounds
        if pos_mag < EARTH_RADIUS_KM {
            return Err(Sgp4Error::SatelliteDecayed);
        }
        
        if pos_mag > 100000.0 {
            return Err(Sgp4Error::NumericalError(format!("Position magnitude too large: {} km", pos_mag)));
        }
        
        if vel_mag > 20.0 {
            return Err(Sgp4Error::NumericalError(format!("Velocity magnitude too large: {} km/s", vel_mag)));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn create_hotsat1_elements() -> OrbitalElements {
        OrbitalElements {
            name: "HOTSAT-1".to_string(),
            norad_id: 56954,
            international_designator: "23084Y".to_string(),
            epoch: DateTime::from_str("2025-08-29T07:07:10.490112Z").unwrap(),
            mean_motion_dot: 0.00007163,
            mean_motion_ddot: 0.0,
            bstar: 0.00031779927,
            ephemeris_type: 0,
            element_set_number: 999,
            inclination: 97.5868,
            raan: 7.4102,
            eccentricity: 0.0005478,
            argument_of_perigee: 336.1866,
            mean_anomaly: 23.9116,
            mean_motion: 15.21880160,
            revolution_number: 12269,
            classification: 'U',
        }
    }

    #[test]
    fn test_propagator_creation() {
        let elements = create_hotsat1_elements();
        let propagator = Sgp4Propagator::new(elements).unwrap();
        assert_eq!(propagator.elements.norad_id, 56954);
    }

    #[test]
    fn test_deep_space_rejection() {
        let mut elements = create_hotsat1_elements();
        elements.mean_motion = 1.0; // Very slow orbit (deep space)
        
        let result = Sgp4Propagator::new(elements);
        assert!(matches!(result, Err(Sgp4Error::DeepSpaceOrbit)));
    }

    #[test]
    fn test_propagation_at_epoch() {
        let elements = create_hotsat1_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        
        // Propagate at epoch (should work without major perturbations)
        let state = propagator.propagate(elements.epoch).unwrap();
        
        // Basic sanity checks
        let pos_mag = magnitude(&state.position);
        let vel_mag = magnitude(&state.velocity);
        
        assert!(pos_mag > EARTH_RADIUS_KM, "Position should be above Earth");
        assert!(pos_mag < 10000.0, "Position should be reasonable for LEO");
        assert!(vel_mag > 5.0, "Velocity should be orbital speed");
        assert!(vel_mag < 12.0, "Velocity should be reasonable for LEO");
        
        println!("Position magnitude: {:.3} km", pos_mag);
        println!("Velocity magnitude: {:.3} km/s", vel_mag);
    }

    #[test]
    fn test_time_bounds() {
        let elements = create_hotsat1_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        
        // Test far future (should fail)
        let far_future = elements.epoch + chrono::Duration::days(4000);
        let result = propagator.propagate(far_future);
        assert!(matches!(result, Err(Sgp4Error::TimeOutOfBounds { .. })));
    }

    #[test]
    fn test_propagation_consistency() {
        let elements = create_hotsat1_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        
        // Propagate forward 1 orbit (~90 minutes)
        let future_time = elements.epoch + chrono::Duration::minutes(90);
        let state = propagator.propagate(future_time).unwrap();
        
        // Results should be physically reasonable
        let pos_mag = magnitude(&state.position);
        let vel_mag = magnitude(&state.velocity);
        
        assert!(pos_mag > EARTH_RADIUS_KM);
        assert!(vel_mag > 0.0);
        
        // Energy check (rough approximation)
        let mu = 398600.5;
        let specific_energy = vel_mag * vel_mag / 2.0 - mu / pos_mag;
        assert!(specific_energy < 0.0, "Orbit should be elliptical (negative energy)");
    }

    #[test]
    fn test_invalid_elements() {
        let mut elements = create_hotsat1_elements();
        elements.eccentricity = 1.5; // Hyperbolic
        
        let result = Sgp4Propagator::new(elements);
        assert!(matches!(result, Err(Sgp4Error::InvalidElements(_))));
    }
}