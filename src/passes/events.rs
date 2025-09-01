//! Horizon event detection algorithms for satellite pass prediction

use chrono::{DateTime, Utc, Duration};
use crate::sgp4::{Sgp4Propagator, StateVector};
use crate::observer::{Observer, TopocentricFrame, LookAngles};
use crate::coordinates::{CoordinateContext, teme_to_gcrs, gcrs_to_itrs, Vector3};

/// Configuration for event detection algorithms
#[derive(Debug, Clone)]
pub struct EventDetectorConfig {
    /// Minimum elevation angle in degrees for pass qualification
    pub min_elevation_deg: f64,
    /// Time resolution for binary search (seconds)
    pub time_resolution_sec: f64,
    /// Maximum iterations for Newton-Raphson refinement
    pub max_refinement_iterations: usize,
    /// Convergence tolerance for Newton-Raphson (seconds)
    pub refinement_tolerance_sec: f64,
    /// Initial time step for coarse search (minutes)
    pub initial_time_step_min: f64,
    /// Fine time step for detailed analysis (seconds)
    pub fine_time_step_sec: f64,
}

impl Default for EventDetectorConfig {
    fn default() -> Self {
        Self {
            min_elevation_deg: 0.0,  // Horizon level
            time_resolution_sec: 1.0, // 1-second accuracy requirement
            max_refinement_iterations: 10,
            refinement_tolerance_sec: 0.1,
            initial_time_step_min: 5.0, // 5-minute coarse steps
            fine_time_step_sec: 30.0, // 30-second fine steps
        }
    }
}

/// Type of satellite visibility event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Satellite rises above horizon (or minimum elevation)
    Rise,
    /// Satellite reaches maximum elevation during pass
    MaxElevation,
    /// Satellite sets below horizon (or minimum elevation)
    Set,
}

/// Horizon event with precise timing and position data
#[derive(Debug, Clone)]
pub struct HorizonEvent {
    /// Type of event
    pub event_type: EventType,
    /// Precise time of event (refined to 1-second accuracy)
    pub time: DateTime<Utc>,
    /// Satellite elevation at event time (degrees)
    pub elevation_deg: f64,
    /// Satellite azimuth at event time (degrees) 
    pub azimuth_deg: f64,
    /// Range to satellite at event time (meters)
    pub range_m: f64,
}

/// Collection of events for a single satellite pass
#[derive(Debug, Clone)]
pub struct PassEvents {
    /// Rise event (satellite appears above minimum elevation)
    pub rise: Option<HorizonEvent>,
    /// Maximum elevation event
    pub max_elevation: Option<HorizonEvent>,
    /// Set event (satellite disappears below minimum elevation)
    pub set: Option<HorizonEvent>,
}

impl PassEvents {
    /// Check if this represents a valid complete pass
    pub fn is_valid_pass(&self) -> bool {
        self.rise.is_some() && self.max_elevation.is_some() && self.set.is_some()
    }
    
    /// Get pass duration in seconds
    pub fn duration_seconds(&self) -> Option<f64> {
        if let (Some(rise), Some(set)) = (&self.rise, &self.set) {
            Some((set.time - rise.time).num_milliseconds() as f64 / 1000.0)
        } else {
            None
        }
    }
    
    /// Get maximum elevation reached during pass
    pub fn max_elevation_deg(&self) -> Option<f64> {
        self.max_elevation.as_ref().map(|event| event.elevation_deg)
    }
}

/// Time window for satellite pass analysis  
#[derive(Debug, Clone)]
pub struct PassWindow {
    /// Start time of analysis window
    pub start: DateTime<Utc>,
    /// End time of analysis window
    pub end: DateTime<Utc>,
}

impl PassWindow {
    /// Create new time window
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, HorizonEventError> {
        if end <= start {
            return Err(HorizonEventError::InvalidTimeWindow(
                "End time must be after start time".to_string()
            ));
        }
        Ok(PassWindow { start, end })
    }
    
    /// Get duration of window in seconds
    pub fn duration_seconds(&self) -> f64 {
        (self.end - self.start).num_milliseconds() as f64 / 1000.0
    }
}

/// Event detection error types
#[derive(Debug, Clone, PartialEq)]
pub enum HorizonEventError {
    /// Invalid time window parameters
    InvalidTimeWindow(String),
    /// Satellite propagation failed
    PropagationError(String),
    /// Observer coordinate transformation failed  
    CoordinateError(String),
    /// Event detection algorithm failed
    DetectionError(String),
    /// Numerical convergence failure
    ConvergenceError(String),
}

impl std::fmt::Display for HorizonEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HorizonEventError::InvalidTimeWindow(msg) => write!(f, "Invalid time window: {}", msg),
            HorizonEventError::PropagationError(msg) => write!(f, "Propagation error: {}", msg),
            HorizonEventError::CoordinateError(msg) => write!(f, "Coordinate error: {}", msg),
            HorizonEventError::DetectionError(msg) => write!(f, "Detection error: {}", msg),
            HorizonEventError::ConvergenceError(msg) => write!(f, "Convergence error: {}", msg),
        }
    }
}

impl std::error::Error for HorizonEventError {}

/// Main event detector for satellite horizon events
pub struct EventDetector {
    /// SGP4 propagator for satellite position calculations
    propagator: Sgp4Propagator,
    /// Observer location and coordinate frame
    observer: TopocentricFrame,
    /// Coordinate context for transformations
    coordinate_context: CoordinateContext,
    /// Configuration parameters
    pub config: EventDetectorConfig,
}

impl EventDetector {
    /// Create new event detector
    pub fn new(
        propagator: Sgp4Propagator,
        observer: Observer,
        coordinate_context: CoordinateContext,
        config: EventDetectorConfig,
    ) -> Self {
        let observer_frame = TopocentricFrame::new(observer);
        
        EventDetector {
            propagator,
            observer: observer_frame,
            coordinate_context,
            config,
        }
    }
    
    /// Find all satellite pass events within the specified time window
    pub fn find_passes(&self, _window: &PassWindow) -> Result<Vec<PassEvents>, HorizonEventError> {
        // Simplified implementation for now
        Ok(vec![])
    }
    
    /// Get satellite elevation at specific time
    pub fn get_elevation_at_time(&self, time: DateTime<Utc>) -> Result<f64, HorizonEventError> {
        let look_angles = self.get_look_angles_at_time(time)?;
        Ok(look_angles.elevation_deg())
    }
    
    /// Get satellite look angles at specific time
    fn get_look_angles_at_time(&self, time: DateTime<Utc>) -> Result<LookAngles, HorizonEventError> {
        // Propagate satellite to specified time
        let state_teme = self.propagator.propagate(time)
            .map_err(|e| HorizonEventError::PropagationError(format!("SGP4 propagation failed: {}", e)))?;
        
        // Convert TEME to GCRS
        let state_gcrs = teme_to_gcrs(&state_teme, &self.coordinate_context)
            .map_err(|e| HorizonEventError::CoordinateError(format!("TEME to GCRS conversion failed: {}", e)))?;
        
        // Convert GCRS to ITRS (Earth-fixed)
        let state_itrs = gcrs_to_itrs(&state_gcrs, &self.coordinate_context)
            .map_err(|e| HorizonEventError::CoordinateError(format!("GCRS to ITRS conversion failed: {}", e)))?;
        
        // Calculate look angles from observer
        let look_angles = self.observer.calculate_look_angles(Vector3::new(state_itrs.position))
            .map_err(|e| HorizonEventError::CoordinateError(format!("Look angle calculation failed: {}", e)))?;
        
        Ok(look_angles)
    }
}