//! Satellite pass prediction functionality
//!
//! This module provides comprehensive satellite pass prediction capabilities including:
//! - Time range scanning for pass detection
//! - Rise/set time calculation with binary search
//! - Maximum elevation computation during passes
//! - Minimum elevation filtering
//! - Integration with SGP4 propagation and observer calculations
//!
//! The main interface is through the `PassPredictor` struct which coordinates
//! orbital propagation with observer-specific calculations to find satellite passes.

pub mod predictor;

pub use predictor::PassPredictor;

/// Represents a complete satellite pass over an observer
#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    /// Rise time when satellite crosses minimum elevation threshold
    pub rise_time: f64,
    /// Time when satellite reaches maximum elevation during pass
    pub max_elevation_time: f64,
    /// Set time when satellite drops below minimum elevation threshold  
    pub set_time: f64,
    /// Maximum elevation angle reached during pass (degrees)
    pub max_elevation: f64,
    /// Azimuth angle at maximum elevation (degrees)
    pub max_elevation_azimuth: f64,
    /// Range to satellite at maximum elevation (km)
    pub max_elevation_range: f64,
}

/// Configuration for pass prediction
#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// Minimum elevation angle to consider for passes (degrees)
    pub min_elevation: f64,
    /// Time step for initial scanning (seconds)
    pub time_step: f64,
    /// Tolerance for binary search convergence (seconds)
    pub time_tolerance: f64,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            min_elevation: 0.0,
            time_step: 60.0, // 1 minute initial step
            time_tolerance: 1.0, // 1 second accuracy
        }
    }
}

/// Pass prediction errors
#[derive(Debug, Clone, PartialEq)]
pub enum PassError {
    /// Error in SGP4 propagation
    PropagationError(String),
    /// Error in observer calculations
    ObserverError(String),
    /// Invalid configuration parameters
    InvalidConfig(String),
    /// Time range or calculation error
    TimeError(String),
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassError::PropagationError(msg) => write!(f, "Propagation error: {}", msg),
            PassError::ObserverError(msg) => write!(f, "Observer error: {}", msg),
            PassError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            PassError::TimeError(msg) => write!(f, "Time error: {}", msg),
        }
    }
}

impl std::error::Error for PassError {}