//! Main pass prediction algorithm implementation
//!
//! This module implements the core pass prediction algorithm that combines
//! SGP4 orbital propagation with observer calculations to find satellite passes.
//! It uses time range scanning with binary search for precise rise/set times.

use super::{Pass, PassError, PredictionConfig};
use crate::sgp4::{Sgp4Propagator, Sgp4Error};
use crate::observer::{Observer, TopocentricFrame, ObserverError};
use crate::coordinates::{teme_to_gcrs, gcrs_to_itrs, CoordinateContext, Vector3};
use chrono::{DateTime, Utc, Duration};

/// Main satellite pass predictor
pub struct PassPredictor {
    /// SGP4 propagator for orbital calculations
    propagator: Sgp4Propagator,
    /// Observer topocentric frame
    observer_frame: TopocentricFrame,
    /// Prediction configuration
    config: PredictionConfig,
    /// Coordinate transformation context (for EOP data)
    coord_context: CoordinateContext,
}

/// Internal state for pass detection during scanning
#[derive(Debug, Clone)]
struct ScanState {
    /// Current time
    time: DateTime<Utc>,
    /// Elevation angle at this time (degrees)
    elevation_deg: f64,
    /// Azimuth angle at this time (degrees)  
    azimuth_deg: f64,
    /// Range to satellite at this time (km)
    range_km: f64,
}

impl PassPredictor {
    /// Create new pass predictor
    ///
    /// # Arguments
    /// * `propagator` - SGP4 propagator for the satellite
    /// * `observer` - Ground observer location
    /// * `coord_context` - Coordinate transformation context with EOP data
    /// * `config` - Optional prediction configuration (uses default if None)
    ///
    /// # Returns
    /// New PassPredictor instance
    pub fn new(
        propagator: Sgp4Propagator,
        observer: Observer,
        coord_context: CoordinateContext,
        config: Option<PredictionConfig>,
    ) -> Self {
        let observer_frame = TopocentricFrame::new(observer);
        let config = config.unwrap_or_default();

        PassPredictor {
            propagator,
            observer_frame,
            config,
            coord_context,
        }
    }

    /// Find all satellite passes over a specified time range
    ///
    /// # Arguments
    /// * `start_time` - Start of search window
    /// * `end_time` - End of search window
    ///
    /// # Returns
    /// Vector of Pass objects representing all passes found
    ///
    /// # Errors
    /// Returns PassError if propagation or computation fails
    pub fn find_passes(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<Pass>, PassError> {
        if start_time >= end_time {
            return Err(PassError::TimeError(
                "Start time must be before end time".to_string(),
            ));
        }

        let mut passes = Vec::new();
        let mut current_time = start_time;
        let mut in_pass = false;
        let mut pass_start: Option<ScanState> = None;
        let mut max_elevation_state: Option<ScanState> = None;

        // Main scanning loop
        while current_time < end_time {
            // Calculate satellite visibility at current time
            let current_state = self.calculate_visibility(current_time)?;

            let above_horizon = current_state.elevation_deg >= self.config.min_elevation;

            if !in_pass && above_horizon {
                // Starting a new pass - find precise rise time
                let rise_time = self.find_rise_time(
                    current_time - Duration::seconds(self.config.time_step as i64),
                    current_time,
                )?;
                let rise_state = self.calculate_visibility(rise_time)?;

                in_pass = true;
                pass_start = Some(rise_state.clone());
                max_elevation_state = Some(rise_state);

                println!("Pass start detected at {}", rise_time);
            } else if in_pass && above_horizon {
                // Continue tracking pass - update maximum elevation if needed
                if let Some(ref max_state) = max_elevation_state {
                    if current_state.elevation_deg > max_state.elevation_deg {
                        max_elevation_state = Some(current_state.clone());
                    }
                }
            } else if in_pass && !above_horizon {
                // End of pass - find precise set time and record the pass
                let set_time = self.find_set_time(
                    current_time - Duration::seconds(self.config.time_step as i64),
                    current_time,
                )?;

                if let (Some(start_state), Some(max_state)) = (pass_start.take(), max_elevation_state.take()) {
                    let pass = Pass {
                        rise_time: start_state.time.timestamp() as f64,
                        max_elevation_time: max_state.time.timestamp() as f64,
                        set_time: set_time.timestamp() as f64,
                        max_elevation: max_state.elevation_deg,
                        max_elevation_azimuth: max_state.azimuth_deg,
                        max_elevation_range: max_state.range_km,
                    };

                    println!(
                        "Pass completed: rise={:.1}°, max={:.1}°, set time={}",
                        start_state.elevation_deg, max_state.elevation_deg, set_time
                    );

                    passes.push(pass);
                }

                in_pass = false;
            }

            current_time = current_time + Duration::seconds(self.config.time_step as i64);
        }

        // Handle pass that might still be ongoing at end of time range
        if in_pass {
            if let (Some(start_state), Some(max_state)) = (pass_start, max_elevation_state) {
                // Use end time as set time for ongoing pass
                let pass = Pass {
                    rise_time: start_state.time.timestamp() as f64,
                    max_elevation_time: max_state.time.timestamp() as f64,
                    set_time: end_time.timestamp() as f64,
                    max_elevation: max_state.elevation_deg,
                    max_elevation_azimuth: max_state.azimuth_deg,
                    max_elevation_range: max_state.range_km,
                };

                passes.push(pass);
                println!("Ongoing pass recorded with end time as set time");
            }
        }

        Ok(passes)
    }

    /// Calculate satellite visibility (elevation, azimuth, range) at given time
    ///
    /// # Arguments
    /// * `time` - Time to calculate visibility for
    ///
    /// # Returns
    /// ScanState with visibility information
    ///
    /// # Errors
    /// Returns PassError if propagation or coordinate transformation fails
    fn calculate_visibility(&self, time: DateTime<Utc>) -> Result<ScanState, PassError> {
        // Propagate satellite position using SGP4
        let teme_state = self.propagator.propagate(time).map_err(|e| match e {
            Sgp4Error::InvalidElements(msg) => PassError::PropagationError(msg),
            Sgp4Error::DeepSpaceOrbit => PassError::PropagationError("Deep space orbit not supported".to_string()),
            Sgp4Error::SatelliteDecayed => PassError::PropagationError("Satellite has decayed".to_string()),
            Sgp4Error::TimeOutOfBounds { days } => PassError::TimeError(format!("Time out of bounds: {} days", days)),
            Sgp4Error::NumericalError(msg) => PassError::PropagationError(format!("Numerical error: {}", msg)),
            Sgp4Error::PropagationError(msg) => PassError::PropagationError(msg),
        })?;

        // Convert TEME state to GCRS
        let gcrs_state = teme_to_gcrs(&teme_state, &self.coord_context)
            .map_err(|e| PassError::PropagationError(format!("TEME to GCRS conversion failed: {:?}", e)))?;

        // Convert GCRS to ITRS (Earth-fixed)
        let itrs_state = gcrs_to_itrs(&gcrs_state, &self.coord_context)
            .map_err(|e| PassError::PropagationError(format!("GCRS to ITRS conversion failed: {:?}", e)))?;

        // Extract position from ITRS state  
        let itrs_pos = Vector3::new(itrs_state.position);

        // Calculate look angles from observer to satellite
        let look_angles = self
            .observer_frame
            .calculate_look_angles(itrs_pos)
            .map_err(|e| match e {
                ObserverError::InvalidCoordinates(msg) => PassError::ObserverError(msg),
                ObserverError::ComputationError(msg) => PassError::ObserverError(msg),
            })?;

        Ok(ScanState {
            time,
            elevation_deg: look_angles.elevation_deg(),
            azimuth_deg: look_angles.azimuth_deg(),
            range_km: look_angles.range_m / 1000.0, // Convert to km
        })
    }

    /// Find precise rise time using binary search
    ///
    /// # Arguments
    /// * `before_time` - Time when satellite was below horizon
    /// * `after_time` - Time when satellite was above horizon
    ///
    /// # Returns
    /// Precise rise time when satellite crosses minimum elevation
    ///
    /// # Errors
    /// Returns PassError if binary search fails
    fn find_rise_time(
        &self,
        before_time: DateTime<Utc>,
        after_time: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, PassError> {
        self.binary_search_horizon_crossing(before_time, after_time, true)
    }

    /// Find precise set time using binary search
    ///
    /// # Arguments
    /// * `before_time` - Time when satellite was above horizon
    /// * `after_time` - Time when satellite was below horizon
    ///
    /// # Returns
    /// Precise set time when satellite crosses minimum elevation
    ///
    /// # Errors
    /// Returns PassError if binary search fails
    fn find_set_time(
        &self,
        before_time: DateTime<Utc>,
        after_time: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, PassError> {
        self.binary_search_horizon_crossing(before_time, after_time, false)
    }

    /// Binary search for horizon crossing (rise or set event)
    ///
    /// # Arguments
    /// * `start_time` - Start of search interval
    /// * `end_time` - End of search interval  
    /// * `finding_rise` - True for rise time, false for set time
    ///
    /// # Returns
    /// Precise time of horizon crossing
    ///
    /// # Errors
    /// Returns PassError if search fails to converge
    fn binary_search_horizon_crossing(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        finding_rise: bool,
    ) -> Result<DateTime<Utc>, PassError> {
        let mut low = start_time;
        let mut high = end_time;
        const MAX_ITERATIONS: i32 = 50;
        let mut iterations = 0;

        while (high - low).num_seconds() > self.config.time_tolerance as i64 && iterations < MAX_ITERATIONS {
            iterations += 1;
            
            // Calculate midpoint
            let duration_between = high - low;
            let mid_duration = duration_between / 2;
            let mid = low + mid_duration;

            // Check elevation at midpoint
            let mid_state = self.calculate_visibility(mid)?;
            let above_horizon = mid_state.elevation_deg >= self.config.min_elevation;

            if finding_rise {
                // For rise: narrow search to where satellite transitions from below to above
                if above_horizon {
                    high = mid;
                } else {
                    low = mid;
                }
            } else {
                // For set: narrow search to where satellite transitions from above to below
                if above_horizon {
                    low = mid;
                } else {
                    high = mid;
                }
            }
        }

        if iterations >= MAX_ITERATIONS {
            return Err(PassError::TimeError(
                format!("Binary search failed to converge after {} iterations", MAX_ITERATIONS)
            ));
        }

        // Return midpoint as best estimate
        let duration_between = high - low;
        let mid_duration = duration_between / 2;
        Ok(low + mid_duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sgp4::{OrbitalElements, OmmParser};
    use crate::eop::EopData;
    use std::str::FromStr;

    fn create_test_elements() -> OrbitalElements {
        let omm_data = r#"{
            "OBJECT_NAME": "HOTSAT-1",
            "OBJECT_ID": "2023-084Y",
            "EPOCH": "2025-08-29T07:07:09.975072",
            "MEAN_MOTION": 15.2188016,
            "ECCENTRICITY": 0.00054784,
            "INCLINATION": 97.5868,
            "RA_OF_ASC_NODE": 7.4102,
            "ARG_OF_PERICENTER": 336.1866,
            "MEAN_ANOMALY": 23.9116,
            "EPHEMERIS_TYPE": 0,
            "CLASSIFICATION_TYPE": "U",
            "NORAD_CAT_ID": 56954,
            "ELEMENT_SET_NO": 999,
            "REV_AT_EPOCH": 12269,
            "BSTAR": 0.00031779927,
            "MEAN_MOTION_DOT": 7.163e-05,
            "MEAN_MOTION_DDOT": 0
        }"#;

        let parser = OmmParser::new();
        parser.parse(omm_data).unwrap()
    }

    fn create_test_observer() -> Observer {
        // Berlin coordinates from fixture
        Observer::new(52.52, 13.405, 34.0).unwrap()
    }

    fn create_test_coord_context() -> CoordinateContext {
        // Create fake EOP data for testing
        let eop_data = EopData {
            mjd: 60000,
            x_pole: 0.05,
            y_pole: 0.3,
            ut1_utc: 0.1,
            lod: 0.001,
            dx_cip: None,
            dy_cip: None,
            dpsi: None,
            deps: None,
        };
        
        let test_time = DateTime::from_str("2025-08-29T07:07:10Z").unwrap();
        CoordinateContext::new(test_time, eop_data).unwrap()
    }

    #[test]
    fn test_predictor_creation() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            None, // Use default config
        );

        assert_eq!(predictor.config.min_elevation, 0.0);
        assert_eq!(predictor.config.time_step, 60.0);
        assert_eq!(predictor.config.time_tolerance, 1.0);
    }

    #[test]
    fn test_predictor_with_custom_config() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let config = PredictionConfig {
            min_elevation: 15.0,
            time_step: 30.0,
            time_tolerance: 0.5,
        };

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            Some(config.clone()),
        );

        assert_eq!(predictor.config.min_elevation, 15.0);
        assert_eq!(predictor.config.time_step, 30.0);
        assert_eq!(predictor.config.time_tolerance, 0.5);
    }

    #[test]
    fn test_calculate_visibility() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            None,
        );

        // Test visibility calculation at epoch
        let state = predictor.calculate_visibility(elements.epoch).unwrap();

        assert_eq!(state.time, elements.epoch);
        assert!(state.elevation_deg.abs() <= 90.0, "Elevation should be within ±90°");
        assert!(state.azimuth_deg >= 0.0 && state.azimuth_deg < 360.0, "Azimuth should be in [0°, 360°)");
        assert!(state.range_km > 300.0, "Range should be reasonable for LEO satellite");

        println!("Visibility at epoch:");
        println!("  Elevation: {:.2}°", state.elevation_deg);
        println!("  Azimuth: {:.2}°", state.azimuth_deg);
        println!("  Range: {:.2} km", state.range_km);
    }

    #[test]
    fn test_find_passes_time_validation() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            None,
        );

        // Test invalid time range
        let start_time = elements.epoch;
        let end_time = start_time - Duration::hours(1); // End before start

        let result = predictor.find_passes(start_time, end_time);
        assert!(result.is_err());
        match result {
            Err(PassError::TimeError(_)) => {}, // Expected
            _ => panic!("Expected TimeError for invalid time range"),
        }
    }

    #[test]
    fn test_find_passes_short_duration() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let config = PredictionConfig {
            min_elevation: 0.0, // Any elevation
            time_step: 300.0,   // 5 minute steps for faster test
            time_tolerance: 5.0, // 5 second tolerance for test
        };

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            Some(config),
        );

        // Test short 1-hour window
        let start_time = elements.epoch;
        let end_time = start_time + Duration::hours(1);

        let passes = predictor.find_passes(start_time, end_time).unwrap();

        // Should complete without error (may or may not find passes in 1 hour)
        println!("Found {} passes in 1-hour window", passes.len());
        
        // Validate any passes found
        for (i, pass) in passes.iter().enumerate() {
            println!("Pass {}: rise={:.1}°, max={:.1}°, range={:.1}km", 
                i + 1, pass.max_elevation, pass.max_elevation, pass.max_elevation_range);
            
            assert!(pass.rise_time <= pass.max_elevation_time, "Rise time should be before max elevation");
            assert!(pass.max_elevation_time <= pass.set_time, "Max elevation should be before set time");
            assert!(pass.max_elevation >= 0.0, "Max elevation should be non-negative");
            assert!(pass.max_elevation <= 90.0, "Max elevation should not exceed 90°");
            assert!(pass.max_elevation_range > 0.0, "Range should be positive");
        }
    }

    #[test]
    fn test_binary_search_horizon_crossing() {
        let elements = create_test_elements();
        let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
        let observer = create_test_observer();
        let coord_context = create_test_coord_context();

        let config = PredictionConfig {
            min_elevation: 10.0, // Test with 10° minimum elevation
            time_step: 60.0,
            time_tolerance: 1.0,
        };

        let predictor = PassPredictor::new(
            propagator,
            observer,
            coord_context,
            Some(config),
        );

        // Find a time when satellite is below 10° and another when above
        let mut test_time = elements.epoch;
        let mut below_time: Option<DateTime<Utc>> = None;
        let mut above_time: Option<DateTime<Utc>> = None;

        // Search for suitable test times (within first 24 hours)
        for _ in 0..1440 { // 1 minute steps for 24 hours
            let state = predictor.calculate_visibility(test_time).unwrap();
            
            if state.elevation_deg < 10.0 && below_time.is_none() {
                below_time = Some(test_time);
            } else if state.elevation_deg >= 10.0 && below_time.is_some() && above_time.is_none() {
                above_time = Some(test_time);
                break;
            }
            
            test_time = test_time + Duration::minutes(1);
        }

        if let (Some(below), Some(above)) = (below_time, above_time) {
            // Test binary search for rise time
            let rise_time = predictor.binary_search_horizon_crossing(below, above, true).unwrap();
            
            // Validate the result
            assert!(rise_time >= below, "Rise time should be after below time");
            assert!(rise_time <= above, "Rise time should be before above time");
            
            // Check that elevation at rise time is close to minimum
            let rise_state = predictor.calculate_visibility(rise_time).unwrap();
            let elevation_diff = (rise_state.elevation_deg - 10.0).abs();
            assert!(elevation_diff < 1.0, "Rise time elevation should be close to minimum: got {:.2}°", rise_state.elevation_deg);
            
            println!("Binary search test:");
            println!("  Below time: {} (elev: {:.2}°)", below, predictor.calculate_visibility(below).unwrap().elevation_deg);
            println!("  Above time: {} (elev: {:.2}°)", above, predictor.calculate_visibility(above).unwrap().elevation_deg);  
            println!("  Rise time: {} (elev: {:.2}°)", rise_time, rise_state.elevation_deg);
        } else {
            println!("Could not find suitable test times for binary search test - this is expected for some satellite positions");
        }
    }
}