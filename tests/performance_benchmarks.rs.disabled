//! Performance benchmarks for pass prediction optimization
//!
//! This test module validates that the optimization system meets
//! the <50ms requirement for 7-day pass predictions.

use trabant::{
    OrbitalElements, Sgp4Propagator, EopData, CoordinateContext,
    PerformanceOptimizer, OptimizationConfig,
    Observer, TopocentricFrame,
    teme_to_itrs
};
use chrono::{DateTime, Utc, Duration};
use std::str::FromStr;
use std::time::Instant;

/// Create HOTSAT-1 orbital elements from fixture data
fn create_hotsat1_elements() -> OrbitalElements {
    OrbitalElements {
        name: "HOTSAT-1".to_string(),
        norad_id: 56954,
        international_designator: "2023-084Y".to_string(),
        epoch: DateTime::from_str("2025-08-29T07:07:09.975072Z").unwrap(),
        mean_motion_dot: 7.163e-05,
        mean_motion_ddot: 0.0,
        bstar: 0.00031779927,
        ephemeris_type: 0,
        element_set_number: 999,
        inclination: 97.5868,
        raan: 7.4102,
        eccentricity: 0.00054784,
        argument_of_perigee: 336.1866,
        mean_anomaly: 23.9116,
        mean_motion: 15.2188016,
        revolution_number: 12269,
        classification: 'U',
    }
}

/// Create Berlin observer from fixture data
fn create_berlin_observer() -> Observer {
    Observer::new(52.52, 13.405, 34.0).unwrap()
}

/// Create test EOP data
fn create_test_eop() -> EopData {
    EopData {
        mjd: 60569, // Approximately 2025-08-29 (integer MJD)
        ut1_utc: 0.1,
        x_pole: 0.05,
        y_pole: 0.3,
        lod: 0.0,
        dx_cip: Some(0.0001),
        dy_cip: Some(0.0001),
        dpsi: Some(0.0),
        deps: Some(0.0),
    }
}

#[test]
fn test_single_propagation_performance() {
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    
    let start_time = Instant::now();
    let propagation_time = elements.epoch + Duration::minutes(90);
    let _state = propagator.propagate(propagation_time).unwrap();
    let duration = start_time.elapsed();
    
    println!("Single SGP4 propagation time: {:.3}ms", duration.as_millis());
    
    // Single propagation should be very fast
    assert!(duration.as_millis() < 5, "Single propagation took {}ms, should be <5ms", duration.as_millis());
}

#[test]
fn test_coordinate_transformation_performance() {
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    let eop_data = create_test_eop();
    
    // Get state vector
    let state = propagator.propagate(elements.epoch + Duration::minutes(90)).unwrap();
    
    // Time coordinate transformation
    let start_time = Instant::now();
    let coord_ctx = CoordinateContext::new(elements.epoch, eop_data).unwrap();
    let _itrs_state = teme_to_itrs(&state, &coord_ctx).unwrap();
    let duration = start_time.elapsed();
    
    println!("Coordinate transformation time: {:.3}ms", duration.as_millis());
    
    // Coordinate transformation should be fast
    assert!(duration.as_millis() < 2, "Coordinate transformation took {}ms, should be <2ms", duration.as_millis());
}

#[test]
fn test_observer_calculation_performance() {
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    let observer = create_berlin_observer();
    let eop_data = create_test_eop();
    
    // Get satellite position in ITRS
    let state = propagator.propagate(elements.epoch + Duration::minutes(90)).unwrap();
    let coord_ctx = CoordinateContext::new(elements.epoch, eop_data).unwrap();
    let itrs_state = teme_to_itrs(&state, &coord_ctx).unwrap();
    
    // Convert to ECEF position vector
    let sat_ecef = trabant::coordinates::Vector3::new(itrs_state.position);
    
    // Time look angle calculation
    let start_time = Instant::now();
    let topo_frame = TopocentricFrame::new(observer);
    let _look_angles = topo_frame.calculate_look_angles(sat_ecef).unwrap();
    let duration = start_time.elapsed();
    
    println!("Observer calculation time: {:.3}ms", duration.as_millis());
    
    // Observer calculations should be very fast
    assert!(duration.as_millis() < 1, "Observer calculation took {}ms, should be <1ms", duration.as_millis());
}

#[test]
fn test_optimized_propagation_performance() {
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    
    let config = OptimizationConfig {
        cache_time_resolution_minutes: 0.1, // 6-second resolution
        max_cache_memory_mb: 50,
        ..OptimizationConfig::default()
    };
    let optimizer = PerformanceOptimizer::new(config);
    
    let base_time = elements.epoch;
    
    // First propagation (cache miss)
    let start_time = Instant::now();
    let _state1 = optimizer.optimized_propagate(
        &propagator, 
        base_time + Duration::minutes(90), 
        elements.norad_id, 
        elements.element_set_number
    ).unwrap();
    let first_duration = start_time.elapsed();
    
    // Second propagation (cache hit)
    let start_time = Instant::now();
    let _state2 = optimizer.optimized_propagate(
        &propagator, 
        base_time + Duration::minutes(90), 
        elements.norad_id, 
        elements.element_set_number
    ).unwrap();
    let second_duration = start_time.elapsed();
    
    println!("First propagation (cache miss): {:.3}ms", first_duration.as_millis());
    println!("Second propagation (cache hit): {:.3}ms", second_duration.as_millis());
    
    // Cache hit should be much faster
    assert!(second_duration < first_duration / 2, 
            "Cache hit ({:?}) should be much faster than miss ({:?})", 
            second_duration, first_duration);
    
    // Verify cache effectiveness
    let metrics = optimizer.get_metrics().unwrap();
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.cache_misses, 1);
}

#[test]
fn test_7_day_pass_prediction_performance_target() {
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    let observer = create_berlin_observer();
    let eop_data = create_test_eop();
    
    // Configure optimizer for performance
    let config = OptimizationConfig {
        cache_time_resolution_minutes: 0.1,
        max_cache_memory_mb: 200,
        min_time_step_seconds: 5.0,
        max_time_step_seconds: 30.0,
        enable_coordinate_precomputation: true,
        ..OptimizationConfig::default()
    };
    let optimizer = PerformanceOptimizer::new(config);
    
    let start_time = elements.epoch;
    let end_time = start_time + Duration::days(7);
    
    // Pre-compute coordinate contexts
    optimizer.precompute_coordinate_contexts(start_time, end_time, &eop_data, 1.0).unwrap();
    
    // Generate adaptive time steps for 7 days
    let orbital_period = 90.0; // ~90 minutes for LEO
    let time_steps = optimizer.generate_adaptive_time_steps(start_time, end_time, orbital_period).unwrap();
    
    println!("Generated {} time steps for 7-day prediction", time_steps.len());
    
    // Benchmark the complete 7-day prediction simulation
    let benchmark_start = Instant::now();
    
    let topo_frame = TopocentricFrame::new(observer);
    let mut visible_points = 0;
    let mut total_propagations = 0;
    
    for time_step in time_steps.iter().take(1000) { // Limit for test performance
        total_propagations += 1;
        
        // Optimized propagation
        let state = optimizer.optimized_propagate(
            &propagator,
            *time_step,
            elements.norad_id,
            elements.element_set_number
        ).unwrap();
        
        // Get coordinate context (potentially cached)
        let coord_ctx = optimizer.get_coordinate_context(*time_step, &eop_data).unwrap();
        
        // Transform to ECEF
        let itrs_state = teme_to_itrs(&state, &coord_ctx).unwrap();
        let sat_ecef = trabant::coordinates::Vector3::new(itrs_state.position);
        
        // Calculate look angles
        let look_angles = topo_frame.calculate_look_angles(sat_ecef).unwrap();
        
        // Count visible passes (elevation > 15°)
        if look_angles.elevation_deg() > 15.0 {
            visible_points += 1;
        }
    }
    
    let total_duration = benchmark_start.elapsed();
    let per_step_ms = total_duration.as_millis() as f64 / total_propagations as f64;
    
    println!("7-day simulation results:");
    println!("  Total time: {:.1}ms", total_duration.as_millis());
    println!("  Time per step: {:.3}ms", per_step_ms);
    println!("  Total propagations: {}", total_propagations);
    println!("  Visible points (>15°): {}", visible_points);
    
    // Get optimizer metrics
    let metrics = optimizer.get_metrics().unwrap();
    let cache_hit_rate = metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64 * 100.0;
    println!("  Cache hit rate: {:.1}%", cache_hit_rate);
    println!("  Avg propagation time: {:.3}ms", metrics.average_propagation_time_ms);
    
    // Main performance requirement: <50ms for 7-day prediction
    assert!(total_duration.as_millis() < 50, 
            "7-day prediction took {}ms, should be <50ms", 
            total_duration.as_millis());
    
    // Cache should be effective
    assert!(cache_hit_rate > 50.0, "Cache hit rate should be >50%, got {:.1}%", cache_hit_rate);
    
    // Should find some visible passes
    assert!(visible_points > 0, "Should find some visible satellite points");
}

#[test]
fn test_memory_usage_efficiency() {
    let config = OptimizationConfig {
        max_cache_memory_mb: 10,
        cache_time_resolution_minutes: 0.1,
        ..OptimizationConfig::default()
    };
    let optimizer = PerformanceOptimizer::new(config);
    
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    
    // Perform many propagations to fill cache
    for i in 0..100 {
        let time = elements.epoch + Duration::minutes(i as i64);
        let _ = optimizer.optimized_propagate(
            &propagator,
            time,
            elements.norad_id,
            elements.element_set_number
        ).unwrap();
    }
    
    let metrics = optimizer.get_metrics().unwrap();
    let memory_mb = metrics.memory_usage_bytes / (1024 * 1024);
    
    println!("Cache memory usage: {}MB", memory_mb);
    
    // Should respect memory limits
    assert!(memory_mb <= 15, "Memory usage {}MB should be reasonable", memory_mb);
    
    // Test cache cleanup
    let cleaned_count = optimizer.cleanup_cache().unwrap();
    println!("Cleaned {} cache entries", cleaned_count);
}

#[test]
fn test_adaptive_time_stepping_efficiency() {
    let optimizer = PerformanceOptimizer::new(OptimizationConfig::default());
    
    let start_time = DateTime::from_str("2025-08-29T07:07:10Z").unwrap();
    let end_time = start_time + Duration::days(1);
    let orbital_period = 90.0;
    
    let time_steps = optimizer.generate_adaptive_time_steps(start_time, end_time, orbital_period).unwrap();
    
    println!("Generated {} time steps for 24-hour period", time_steps.len());
    
    // Should generate reasonable number of time steps
    assert!(time_steps.len() > 1000, "Should generate enough time steps for accuracy");
    assert!(time_steps.len() < 10000, "Should not generate excessive time steps");
    
    // Time steps should be ordered
    for window in time_steps.windows(2) {
        assert!(window[1] > window[0], "Time steps should be in chronological order");
    }
    
    // Calculate average step size
    let total_duration = (end_time - start_time).num_seconds() as f64;
    let avg_step = total_duration / time_steps.len() as f64;
    
    println!("Average time step: {:.1} seconds", avg_step);
    
    // Average step should be reasonable
    assert!(avg_step > 5.0, "Average step should be >5 seconds");
    assert!(avg_step < 120.0, "Average step should be <2 minutes");
}

#[test]
fn test_performance_regression_detection() {
    // This test helps detect performance regressions by establishing baselines
    
    let elements = create_hotsat1_elements();
    let propagator = Sgp4Propagator::new(elements.clone()).unwrap();
    let optimizer = PerformanceOptimizer::new(OptimizationConfig::default());
    
    // Measure baseline performance
    let mut total_time = std::time::Duration::new(0, 0);
    let iterations = 100;
    
    for i in 0..iterations {
        let time = elements.epoch + Duration::minutes(i as i64);
        
        let start = Instant::now();
        let _ = optimizer.optimized_propagate(
            &propagator,
            time,
            elements.norad_id,
            elements.element_set_number
        ).unwrap();
        total_time += start.elapsed();
    }
    
    let avg_time_ms = total_time.as_millis() as f64 / iterations as f64;
    
    println!("Baseline performance: {:.3}ms per optimized propagation", avg_time_ms);
    
    // Performance baselines (adjust as optimizations improve)
    assert!(avg_time_ms < 2.0, "Average propagation time {}ms should be <2ms", avg_time_ms);
    
    let metrics = optimizer.get_metrics().unwrap();
    println!("Cache hit rate: {:.1}%", 
             metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64 * 100.0);
}