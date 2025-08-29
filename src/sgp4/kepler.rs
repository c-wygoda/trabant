//! Kepler equation solver for SGP4 orbital mechanics
//!
//! This module provides functionality to solve Kepler's equation:
//! M = E - e*sin(E)
//! 
//! Where:
//! - M is the mean anomaly
//! - E is the eccentric anomaly  
//! - e is the orbital eccentricity
//!
//! The solution provides both eccentric and true anomalies needed for position calculations.

use crate::sgp4::constants::{EPSILON, MAX_ITERATIONS, TWO_PI};
use crate::sgp4::math::normalize_angle;

/// Solution to Kepler's equation containing both anomalies
#[derive(Debug, Clone, PartialEq)]
pub struct KeplerSolution {
    /// Eccentric anomaly in radians [0, 2π)
    pub eccentric_anomaly: f64,
    /// True anomaly in radians [0, 2π)  
    pub true_anomaly: f64,
}

/// Solve Kepler's equation for eccentric and true anomalies
/// 
/// Uses Newton-Raphson iteration with smart initial guess selection
/// based on orbital eccentricity.
/// 
/// # Arguments
/// * `mean_anomaly` - Mean anomaly in radians
/// * `eccentricity` - Orbital eccentricity [0, 1)
/// 
/// # Returns
/// KeplerSolution containing both eccentric and true anomalies
/// 
/// # Examples
/// ```
/// use trabant::sgp4::kepler::solve_kepler;
/// 
/// // Circular orbit (e = 0)
/// let sol = solve_kepler(1.0, 0.0);
/// assert!((sol.eccentric_anomaly - 1.0).abs() < 1e-12);
/// assert!((sol.true_anomaly - 1.0).abs() < 1e-12);
/// 
/// // Elliptical orbit
/// let sol = solve_kepler(0.5, 0.1);
/// // For small eccentricity, E ≈ M
/// assert!((sol.eccentric_anomaly - 0.5).abs() < 0.1);
/// ```
pub fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> KeplerSolution {
    // Normalize mean anomaly to [0, 2π)
    let mean_anomaly = normalize_angle(mean_anomaly);
    
    // Handle circular orbit case directly
    if eccentricity < EPSILON {
        return KeplerSolution {
            eccentric_anomaly: mean_anomaly,
            true_anomaly: mean_anomaly,
        };
    }
    
    // Clamp eccentricity to prevent numerical issues
    let eccentricity = eccentricity.min(0.9999);
    
    // Smart initial guess based on eccentricity
    let mut eccentric_anomaly = initial_guess(mean_anomaly, eccentricity);
    
    // Newton-Raphson iteration
    for _ in 0..MAX_ITERATIONS {
        let sin_e = eccentric_anomaly.sin();
        let cos_e = eccentric_anomaly.cos();
        
        // Kepler's equation residual: f(E) = E - e*sin(E) - M
        let f = eccentric_anomaly - eccentricity * sin_e - mean_anomaly;
        
        // Derivative: f'(E) = 1 - e*cos(E)
        let df = 1.0 - eccentricity * cos_e;
        
        // Newton step: E_{n+1} = E_n - f(E_n) / f'(E_n)
        let delta = f / df;
        eccentric_anomaly -= delta;
        
        // Normalize to [0, 2π)
        eccentric_anomaly = normalize_angle(eccentric_anomaly);
        
        // Check convergence
        if delta.abs() < EPSILON {
            break;
        }
    }
    
    // Compute true anomaly from eccentric anomaly
    let true_anomaly = eccentric_to_true_anomaly(eccentric_anomaly, eccentricity);
    
    KeplerSolution {
        eccentric_anomaly,
        true_anomaly,
    }
}

/// Generate smart initial guess for eccentric anomaly
fn initial_guess(mean_anomaly: f64, eccentricity: f64) -> f64 {
    if eccentricity < 0.1 {
        // For small eccentricity, E ≈ M
        mean_anomaly
    } else if eccentricity < 0.9 {
        // For moderate eccentricity, use first-order correction
        mean_anomaly + eccentricity * mean_anomaly.sin()
    } else {
        // For high eccentricity, use more sophisticated guess
        if mean_anomaly < std::f64::consts::PI {
            mean_anomaly + 0.85 * eccentricity
        } else {
            mean_anomaly - 0.85 * eccentricity
        }
    }
}

/// Convert eccentric anomaly to true anomaly
/// 
/// Uses the numerically stable formulation:
/// ν = 2 * atan2(√(1+e) * sin(E/2), √(1-e) * cos(E/2))
fn eccentric_to_true_anomaly(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let half_e = eccentric_anomaly * 0.5;
    let sqrt_1_plus_e = (1.0 + eccentricity).sqrt();
    let sqrt_1_minus_e = (1.0 - eccentricity).sqrt();
    
    let numerator = sqrt_1_plus_e * half_e.sin();
    let denominator = sqrt_1_minus_e * half_e.cos();
    
    let true_anomaly = 2.0 * numerator.atan2(denominator);
    
    // Ensure result is in [0, 2π)
    normalize_angle(true_anomaly)
}

/// Alternative method to compute true anomaly using direct trigonometry
/// 
/// For validation and cross-checking purposes
#[allow(dead_code)]
fn eccentric_to_true_anomaly_alt(eccentric_anomaly: f64, eccentricity: f64) -> f64 {
    let cos_e = eccentric_anomaly.cos();
    let sin_e = eccentric_anomaly.sin();
    
    // cos(ν) = (cos(E) - e) / (1 - e*cos(E))
    let cos_nu = (cos_e - eccentricity) / (1.0 - eccentricity * cos_e);
    
    // sin(ν) = √(1-e²) * sin(E) / (1 - e*cos(E))
    let sin_nu = (1.0 - eccentricity * eccentricity).sqrt() * sin_e / (1.0 - eccentricity * cos_e);
    
    normalize_angle(sin_nu.atan2(cos_nu))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_circular_orbit() {
        let solution = solve_kepler(1.0, 0.0);
        assert!((solution.eccentric_anomaly - 1.0).abs() < EPSILON);
        assert!((solution.true_anomaly - 1.0).abs() < EPSILON);
        
        let solution = solve_kepler(PI, 0.0);
        assert!((solution.eccentric_anomaly - PI).abs() < EPSILON);
        assert!((solution.true_anomaly - PI).abs() < EPSILON);
    }

    #[test] 
    fn test_small_eccentricity() {
        let solution = solve_kepler(0.5, 0.1);
        
        // For small e, E ≈ M + e*sin(M)
        let expected_e = 0.5 + 0.1 * 0.5_f64.sin();
        assert!((solution.eccentric_anomaly - expected_e).abs() < 0.01);
        
        // Verify Kepler equation residual
        let residual = solution.eccentric_anomaly - 0.1 * solution.eccentric_anomaly.sin() - 0.5;
        assert!(residual.abs() < EPSILON);
    }

    #[test]
    fn test_moderate_eccentricity() {
        let solution = solve_kepler(1.0, 0.5);
        
        // Verify Kepler equation is satisfied
        let residual = solution.eccentric_anomaly - 0.5 * solution.eccentric_anomaly.sin() - 1.0;
        assert!(residual.abs() < EPSILON);
        
        // True anomaly should be larger than eccentric anomaly for this case
        assert!(solution.true_anomaly > solution.eccentric_anomaly);
    }

    #[test]
    fn test_high_eccentricity() {
        let solution = solve_kepler(0.1, 0.9);
        
        // Verify Kepler equation is satisfied
        let residual = solution.eccentric_anomaly - 0.9 * solution.eccentric_anomaly.sin() - 0.1;
        assert!(residual.abs() < EPSILON);
        
        // For high eccentricity and small M, true anomaly should be much larger
        assert!(solution.true_anomaly > solution.eccentric_anomaly);
    }

    #[test]
    fn test_angle_wrapping() {
        // Test with mean anomaly > 2π
        let solution1 = solve_kepler(0.5, 0.1);
        let solution2 = solve_kepler(0.5 + TWO_PI, 0.1);
        
        assert!((solution1.eccentric_anomaly - solution2.eccentric_anomaly).abs() < EPSILON);
        assert!((solution1.true_anomaly - solution2.true_anomaly).abs() < EPSILON);
    }

    #[test]
    fn test_analytical_solutions() {
        // Test case where analytical solution is known
        // For e = 0.5, M = π, analytical E ≈ 2.3
        let solution = solve_kepler(PI, 0.5);
        
        // Verify the solution satisfies Kepler's equation
        let residual = solution.eccentric_anomaly - 0.5 * solution.eccentric_anomaly.sin() - PI;
        assert!(residual.abs() < EPSILON);
    }

    #[test]
    fn test_convergence() {
        // Test that algorithm converges for various cases
        let test_cases = [
            (0.0, 0.0), (0.5, 0.1), (1.0, 0.3), (2.0, 0.7), (3.0, 0.9),
            (PI, 0.2), (1.5 * PI, 0.5), (2.0 * PI - 0.1, 0.8)
        ];
        
        for (mean_anomaly, eccentricity) in test_cases.iter() {
            let solution = solve_kepler(*mean_anomaly, *eccentricity);
            
            // Verify Kepler equation residual
            let residual = solution.eccentric_anomaly 
                - eccentricity * solution.eccentric_anomaly.sin() 
                - normalize_angle(*mean_anomaly);
            assert!(residual.abs() < EPSILON, 
                "Failed for M={}, e={}, residual={}", mean_anomaly, eccentricity, residual);
            
            // Verify angles are normalized
            assert!(solution.eccentric_anomaly >= 0.0 && solution.eccentric_anomaly < TWO_PI);
            assert!(solution.true_anomaly >= 0.0 && solution.true_anomaly < TWO_PI);
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test very small eccentricity
        let solution = solve_kepler(1.0, 1e-10);
        assert!((solution.eccentric_anomaly - 1.0).abs() < 1e-9);
        assert!((solution.true_anomaly - 1.0).abs() < 1e-9);
        
        // Test near-parabolic case
        let solution = solve_kepler(0.1, 0.9999);
        let residual = solution.eccentric_anomaly - 0.9999 * solution.eccentric_anomaly.sin() - 0.1;
        assert!(residual.abs() < EPSILON);
    }
}