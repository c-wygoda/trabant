//! Performance optimization systems for satellite pass prediction
//!
//! This module implements caching, pre-computation, and adaptive algorithms
//! to achieve <50ms performance for 7-day pass predictions.

use crate::{StateVector, Sgp4Propagator, CoordinateContext, Sgp4Error, EopData};
use chrono::{DateTime, Utc, Duration};
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Performance optimization errors
#[derive(Error, Debug)]
pub enum OptimizationError {
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Memory limit exceeded: {current}MB > {limit}MB")]
    MemoryLimitExceeded { current: usize, limit: usize },
    
    #[error("Time step optimization failed: {0}")]
    TimeStepError(String),
    
    #[error("Propagation error: {0}")]
    PropagationError(#[from] Sgp4Error),
}

/// Cache key for SGP4 propagation results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct PropagationCacheKey {
    /// Satellite NORAD ID
    norad_id: u32,
    /// Time in minutes since epoch (rounded for cache efficiency)
    time_minutes: i64,
    /// Element set number for cache invalidation
    element_set: u32,
}

/// Cached propagation result with timestamp
#[derive(Debug, Clone)]
struct CachedPropagationResult {
    /// Propagated state vector
    state: StateVector,
    /// Time when result was cached
    cached_at: DateTime<Utc>,
    /// Actual propagation time (for interpolation)
    propagation_time: DateTime<Utc>,
}

/// Configuration for optimization system
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Maximum cache memory in MB
    pub max_cache_memory_mb: usize,
    /// Cache entry TTL in hours
    pub cache_ttl_hours: u32,
    /// Time resolution for cache keys (minutes)
    pub cache_time_resolution_minutes: f64,
    /// Minimum time step for adaptive stepping (seconds)
    pub min_time_step_seconds: f64,
    /// Maximum time step for adaptive stepping (seconds)
    pub max_time_step_seconds: f64,
    /// Enable coordinate transformation pre-computation
    pub enable_coordinate_precomputation: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_cache_memory_mb: 100,
            cache_ttl_hours: 24,
            cache_time_resolution_minutes: 0.1, // 6 second resolution
            min_time_step_seconds: 1.0,
            max_time_step_seconds: 60.0,
            enable_coordinate_precomputation: true,
        }
    }
}

/// Main performance optimization system
pub struct PerformanceOptimizer {
    /// Configuration parameters
    config: OptimizationConfig,
    
    /// Propagation result cache
    propagation_cache: Arc<RwLock<HashMap<PropagationCacheKey, CachedPropagationResult>>>,
    
    /// Pre-computed coordinate transformation cache
    coordinate_cache: Arc<RwLock<BTreeMap<i64, CoordinateContext>>>,
    
    /// Cache memory usage tracking
    cache_memory_bytes: Arc<RwLock<usize>>,
    
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Performance tracking metrics
#[derive(Debug, Default, Clone)]
pub struct PerformanceMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_propagations: u64,
    pub memory_usage_bytes: usize,
    pub average_propagation_time_ms: f64,
    pub last_cleanup_time: Option<DateTime<Utc>>,
}

impl PerformanceOptimizer {
    /// Create new performance optimizer with given configuration
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            config,
            propagation_cache: Arc::new(RwLock::new(HashMap::new())),
            coordinate_cache: Arc::new(RwLock::new(BTreeMap::new())),
            cache_memory_bytes: Arc::new(RwLock::new(0)),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Optimized propagation with caching
    pub fn optimized_propagate(
        &self,
        propagator: &Sgp4Propagator,
        time: DateTime<Utc>,
        norad_id: u32,
        element_set: u32,
    ) -> Result<StateVector, OptimizationError> {
        let start_time = std::time::Instant::now();
        
        // Generate cache key
        let cache_key = self.generate_cache_key(norad_id, time, element_set);
        
        // Try cache first
        if let Some(cached_result) = self.get_cached_propagation(&cache_key)? {
            // Check if we need interpolation
            if (cached_result.propagation_time - time).num_seconds().abs() < 1 {
                self.record_cache_hit();
                return Ok(cached_result.state);
            }
            
            // If time difference is small, use interpolation
            if (cached_result.propagation_time - time).num_seconds().abs() < 30 {
                if let Ok(interpolated) = self.interpolate_state(&cached_result.state, 
                                                                cached_result.propagation_time, 
                                                                time) {
                    self.record_cache_hit();
                    return Ok(interpolated);
                }
            }
        }
        
        // Cache miss - perform propagation
        self.record_cache_miss();
        let state = propagator.propagate(time)?;
        
        // Cache the result
        let cached_result = CachedPropagationResult {
            state: state.clone(),
            cached_at: Utc::now(),
            propagation_time: time,
        };
        
        self.cache_propagation_result(cache_key, cached_result)?;
        
        // Record metrics
        let duration = start_time.elapsed();
        self.record_propagation_time(duration.as_millis() as f64);
        
        Ok(state)
    }

    /// Generate optimized time steps for pass prediction
    pub fn generate_adaptive_time_steps(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        orbital_period_minutes: f64,
    ) -> Result<Vec<DateTime<Utc>>, OptimizationError> {
        let mut time_steps = Vec::new();
        let mut current_time = start_time;
        
        // Calculate adaptive step size based on orbital dynamics
        let base_step_seconds = (orbital_period_minutes * 60.0 / 360.0).min(self.config.max_time_step_seconds)
                                                                        .max(self.config.min_time_step_seconds);
        
        while current_time <= end_time {
            time_steps.push(current_time);
            
            // Adaptive step size - smaller steps near horizon crossings
            let step_seconds = self.calculate_adaptive_step(current_time, base_step_seconds)?;
            current_time = current_time + Duration::seconds(step_seconds as i64);
        }
        
        // Ensure we include the end time
        if time_steps.last() != Some(&end_time) {
            time_steps.push(end_time);
        }
        
        Ok(time_steps)
    }

    /// Pre-compute coordinate transformation contexts for a time range
    pub fn precompute_coordinate_contexts(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        eop_data: &EopData,
        time_step_hours: f64,
    ) -> Result<(), OptimizationError> {
        if !self.config.enable_coordinate_precomputation {
            return Ok(());
        }
        
        let mut current_time = start_time;
        let step_duration = Duration::seconds((time_step_hours * 3600.0) as i64);
        
        let mut coord_cache = self.coordinate_cache.write()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        while current_time <= end_time {
            let key = self.time_to_cache_key(current_time);
            
            if !coord_cache.contains_key(&key) {
                match CoordinateContext::new(current_time, eop_data.clone()) {
                    Ok(context) => {
                        coord_cache.insert(key, context);
                    }
                    Err(e) => {
                        // Log warning but continue
                        eprintln!("Warning: Failed to create coordinate context for {}: {}", current_time, e);
                    }
                }
            }
            
            current_time = current_time + step_duration;
        }
        
        Ok(())
    }

    /// Get pre-computed coordinate context or create new one
    pub fn get_coordinate_context(
        &self,
        time: DateTime<Utc>,
        eop_data: &EopData,
    ) -> Result<CoordinateContext, OptimizationError> {
        if !self.config.enable_coordinate_precomputation {
            return CoordinateContext::new(time, eop_data.clone())
                .map_err(|e| OptimizationError::CacheError(format!("Coordinate context error: {}", e)));
        }
        
        let key = self.time_to_cache_key(time);
        
        let coord_cache = self.coordinate_cache.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        if let Some(context) = coord_cache.get(&key) {
            return Ok(context.clone());
        }
        
        // Fallback to dynamic creation
        drop(coord_cache);
        let context = CoordinateContext::new(time, eop_data.clone())
            .map_err(|e| OptimizationError::CacheError(format!("Coordinate context error: {}", e)))?;
        
        // Cache for future use
        let mut coord_cache = self.coordinate_cache.write()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        coord_cache.insert(key, context.clone());
        
        Ok(context)
    }

    /// Perform cache cleanup to manage memory usage
    pub fn cleanup_cache(&self) -> Result<usize, OptimizationError> {
        let now = Utc::now();
        let ttl = Duration::hours(self.config.cache_ttl_hours as i64);
        let mut cleaned_count = 0;
        
        // Clean propagation cache
        {
            let mut prop_cache = self.propagation_cache.write()
                .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
            
            let mut keys_to_remove = Vec::new();
            
            for (key, result) in prop_cache.iter() {
                if now.signed_duration_since(result.cached_at) > ttl {
                    keys_to_remove.push(key.clone());
                }
            }
            
            for key in keys_to_remove {
                prop_cache.remove(&key);
                cleaned_count += 1;
            }
        }
        
        // Clean coordinate cache (keep recent entries)
        {
            let mut coord_cache = self.coordinate_cache.write()
                .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
            
            let cutoff_time = now - ttl;
            let cutoff_key = self.time_to_cache_key(cutoff_time);
            
            let old_keys: Vec<i64> = coord_cache.range(..cutoff_key).map(|(k, _)| *k).collect();
            
            for key in old_keys {
                coord_cache.remove(&key);
                cleaned_count += 1;
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write()
                .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
            metrics.last_cleanup_time = Some(now);
        }
        
        self.update_memory_usage()?;
        
        Ok(cleaned_count)
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> Result<PerformanceMetrics, OptimizationError> {
        let metrics = self.metrics.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        let mut result = (*metrics).clone();
        
        let memory_usage = *self.cache_memory_bytes.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        result.memory_usage_bytes = memory_usage;
        
        Ok(result)
    }

    // Private helper methods
    
    fn generate_cache_key(&self, norad_id: u32, time: DateTime<Utc>, element_set: u32) -> PropagationCacheKey {
        let minutes_since_epoch = time.timestamp() / 60;
        let rounded_minutes = (minutes_since_epoch as f64 / self.config.cache_time_resolution_minutes).round() as i64
                              * self.config.cache_time_resolution_minutes as i64;
        
        PropagationCacheKey {
            norad_id,
            time_minutes: rounded_minutes,
            element_set,
        }
    }
    
    fn get_cached_propagation(&self, key: &PropagationCacheKey) -> Result<Option<CachedPropagationResult>, OptimizationError> {
        let cache = self.propagation_cache.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        Ok(cache.get(key).cloned())
    }
    
    fn cache_propagation_result(&self, key: PropagationCacheKey, result: CachedPropagationResult) -> Result<(), OptimizationError> {
        let mut cache = self.propagation_cache.write()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        cache.insert(key, result);
        
        drop(cache);
        self.update_memory_usage()?;
        self.check_memory_limit()?;
        
        Ok(())
    }
    
    fn interpolate_state(&self, state: &StateVector, from_time: DateTime<Utc>, to_time: DateTime<Utc>) -> Result<StateVector, OptimizationError> {
        let time_diff = (to_time - from_time).num_seconds() as f64;
        
        // Simple linear interpolation for small time differences
        if time_diff.abs() > 30.0 {
            return Err(OptimizationError::TimeStepError("Time difference too large for interpolation".to_string()));
        }
        
        // For small time differences, use velocity to extrapolate position
        Ok(StateVector {
            position: [
                state.position[0] + state.velocity[0] * time_diff,
                state.position[1] + state.velocity[1] * time_diff,
                state.position[2] + state.velocity[2] * time_diff,
            ],
            velocity: state.velocity, // Assume velocity is approximately constant for short intervals
        })
    }
    
    fn calculate_adaptive_step(&self, _current_time: DateTime<Utc>, base_step: f64) -> Result<f64, OptimizationError> {
        // For now, return base step. In full implementation, would analyze:
        // - Orbital position (smaller steps near perigee)
        // - Elevation rate (smaller steps near horizon crossings)
        // - Previous prediction accuracy
        Ok(base_step)
    }
    
    fn time_to_cache_key(&self, time: DateTime<Utc>) -> i64 {
        time.timestamp() / 3600 // Hour resolution for coordinate contexts
    }
    
    fn update_memory_usage(&self) -> Result<(), OptimizationError> {
        let prop_cache = self.propagation_cache.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        let coord_cache = self.coordinate_cache.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        // Rough memory calculation (in bytes)
        let prop_memory = prop_cache.len() * std::mem::size_of::<(PropagationCacheKey, CachedPropagationResult)>();
        let coord_memory = coord_cache.len() * std::mem::size_of::<(i64, CoordinateContext)>();
        
        let total_memory = prop_memory + coord_memory;
        
        let mut memory_usage = self.cache_memory_bytes.write()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        *memory_usage = total_memory;
        
        Ok(())
    }
    
    fn check_memory_limit(&self) -> Result<(), OptimizationError> {
        let memory_usage = *self.cache_memory_bytes.read()
            .map_err(|e| OptimizationError::CacheError(format!("Lock error: {}", e)))?;
        
        let limit_bytes = self.config.max_cache_memory_mb * 1024 * 1024;
        
        if memory_usage > limit_bytes {
            return Err(OptimizationError::MemoryLimitExceeded {
                current: memory_usage / (1024 * 1024),
                limit: self.config.max_cache_memory_mb,
            });
        }
        
        Ok(())
    }
    
    fn record_cache_hit(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.cache_hits += 1;
        }
    }
    
    fn record_cache_miss(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.cache_misses += 1;
        }
    }
    
    fn record_propagation_time(&self, time_ms: f64) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_propagations += 1;
            
            // Update rolling average
            let count = metrics.total_propagations as f64;
            metrics.average_propagation_time_ms = 
                (metrics.average_propagation_time_ms * (count - 1.0) + time_ms) / count;
        }
    }
}