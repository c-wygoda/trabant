//! Satellite pass prediction module
//!
//! This module provides functionality for predicting satellite passes over ground stations,
//! including optimization systems for performance-critical applications.

pub mod optimization;

pub use optimization::{PerformanceOptimizer, OptimizationConfig, OptimizationError, PerformanceMetrics};