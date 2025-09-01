//! Integration tests for the pass prediction system

use trabant::{
    PassPredictor, Pass, PassError, PredictionConfig,
    Sgp4Propagator, OmmParser, Observer, CoordinateContext, EopData
};
use chrono::{DateTime, Utc, Duration};
use std::str::FromStr;

#[test]
fn test_pass_predictor_integration() {
    // Create test orbital elements for HOTSAT-1
    let omm_data = r#"{
        "OBJECT_NAME": "HOTSAT-1",
        "OBJECT_ID": "2023-084Y",
        "EPOCH": "2025-08-29T07:07:09.975072Z",
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

    let elements = OmmParser::parse(omm_data).unwrap();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();

    // Create Berlin observer
    let observer = Observer::new(52.52, 13.405, 34.0).unwrap();

    // Create coordinate context with fake EOP data
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
    let coord_context = CoordinateContext::new(test_time, eop_data).unwrap();

    // Create predictor with configuration for testing
    let config = PredictionConfig {
        min_elevation: 0.0, // Any elevation to catch more passes
        time_step: 300.0,   // 5 minute steps
        time_tolerance: 5.0, // 5 second tolerance
    };

    let predictor = PassPredictor::new(
        propagator,
        observer,
        coord_context,
        Some(config),
    );

    // Test prediction over 1-hour window
    let start_time = elements.epoch;
    let end_time = start_time + Duration::hours(1);

    let passes = predictor.find_passes(start_time, end_time).unwrap();

    println!("Found {} passes in 1-hour test window", passes.len());

    // Basic validation - any passes found should be valid
    for (i, pass) in passes.iter().enumerate() {
        println!("Pass {}: rise={:.1}s, max_elev={:.1}°, set={:.1}s", 
            i + 1, pass.rise_time, pass.max_elevation, pass.set_time);
        
        assert!(pass.rise_time <= pass.max_elevation_time);
        assert!(pass.max_elevation_time <= pass.set_time);
        assert!(pass.max_elevation >= 0.0);
        assert!(pass.max_elevation <= 90.0);
        assert!(pass.max_elevation_range > 0.0);
    }

    // Test should complete successfully regardless of pass count
}

#[test] 
fn test_pass_predictor_time_validation() {
    // Create minimal setup
    let omm_data = r#"{
        "OBJECT_NAME": "TEST-SAT",
        "OBJECT_ID": "TEST-ID",
        "EPOCH": "2025-08-29T07:07:09.975072Z",
        "MEAN_MOTION": 15.0,
        "ECCENTRICITY": 0.001,
        "INCLINATION": 90.0,
        "RA_OF_ASC_NODE": 0.0,
        "ARG_OF_PERICENTER": 0.0,
        "MEAN_ANOMALY": 0.0,
        "EPHEMERIS_TYPE": 0,
        "CLASSIFICATION_TYPE": "U",
        "NORAD_CAT_ID": 12345,
        "ELEMENT_SET_NO": 1,
        "REV_AT_EPOCH": 1,
        "BSTAR": 0.0001,
        "MEAN_MOTION_DOT": 0.0,
        "MEAN_MOTION_DDOT": 0
    }"#;

    let elements = OmmParser::parse(omm_data).unwrap();
    let propagator = Sgp4Propagator::new(elements).unwrap();
    let observer = Observer::new(0.0, 0.0, 0.0).unwrap();
    
    let eop_data = EopData {
        mjd: 60000,
        x_pole: 0.0,
        y_pole: 0.0,
        ut1_utc: 0.0,
        lod: 0.0,
        dx_cip: None,
        dy_cip: None,
        dpsi: None,
        deps: None,
    };
    let test_time = DateTime::from_str("2025-08-29T07:07:10Z").unwrap();
    let coord_context = CoordinateContext::new(test_time, eop_data).unwrap();

    let predictor = PassPredictor::new(propagator, observer, coord_context, None);

    let start_time = DateTime::from_str("2025-08-29T12:00:00Z").unwrap();
    let end_time = start_time - Duration::hours(1); // End before start - invalid

    let result = predictor.find_passes(start_time, end_time);
    assert!(result.is_err());
    match result.unwrap_err() {
        PassError::TimeError(_) => {}, // Expected
        _ => panic!("Expected TimeError for invalid time range"),
    }
}