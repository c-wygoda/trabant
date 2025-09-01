//! Satellite pass prediction and horizon event detection
//!
//! This module provides functionality for:
//! - Detecting satellite pass events (rise, set, maximum elevation)
//! - Binary search algorithms for precise horizon crossing detection
//! - Newton-Raphson refinement for 1-second accuracy requirements
//! - Support for minimum elevation constraints
//! - Adaptive time stepping for computational efficiency

pub mod events;
pub mod predictor;

pub use events::{
    HorizonEvent, EventType, EventDetector, EventDetectorConfig,
    HorizonEventError, PassWindow, PassEvents,
};

pub use predictor::{
    PassPredictor, PassPredictionSummary,
};

/// Pass prediction error types
#[derive(Debug, Clone, PartialEq)]
pub enum PassError {
    /// Invalid time window
    InvalidTimeWindow(String),
    /// Observer or satellite positioning error
    PositionError(String),
    /// Event detection failed
    EventDetectionError(String),
    /// No passes found in time window
    NoPassesFound,
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassError::InvalidTimeWindow(msg) => write!(f, "Invalid time window: {}", msg),
            PassError::PositionError(msg) => write!(f, "Position error: {}", msg),
            PassError::EventDetectionError(msg) => write!(f, "Event detection error: {}", msg),
            PassError::NoPassesFound => write!(f, "No passes found in specified time window"),
        }
    }
}

impl std::error::Error for PassError {}