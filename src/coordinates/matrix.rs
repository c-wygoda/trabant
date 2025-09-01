//! 3x3 matrix operations for coordinate transformations
//!
//! Optimized matrix operations for coordinate frame transformations.
//! All matrices are stored in row-major order.

use std::ops::{Add, Mul};

/// 3x3 matrix for coordinate transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3 {
    pub data: [[f64; 3]; 3],
}

/// 3-element vector for positions and velocities
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub data: [f64; 3],
}

impl Matrix3 {
    /// Create new matrix from 3x3 array
    pub fn new(data: [[f64; 3]; 3]) -> Self {
        Matrix3 { data }
    }

    /// Create identity matrix
    pub fn identity() -> Self {
        Matrix3 {
            data: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create zero matrix
    pub fn zero() -> Self {
        Matrix3 {
            data: [[0.0; 3]; 3],
        }
    }

    /// Create rotation matrix around X-axis
    pub fn rotation_x(angle_rad: f64) -> Self {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        Matrix3 {
            data: [
                [1.0, 0.0, 0.0],
                [0.0, cos_a, -sin_a],
                [0.0, sin_a, cos_a],
            ],
        }
    }

    /// Create rotation matrix around Y-axis  
    pub fn rotation_y(angle_rad: f64) -> Self {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        Matrix3 {
            data: [
                [cos_a, 0.0, sin_a],
                [0.0, 1.0, 0.0],
                [-sin_a, 0.0, cos_a],
            ],
        }
    }

    /// Create rotation matrix around Z-axis
    pub fn rotation_z(angle_rad: f64) -> Self {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        Matrix3 {
            data: [
                [cos_a, -sin_a, 0.0],
                [sin_a, cos_a, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    /// Get transpose of matrix
    pub fn transpose(&self) -> Self {
        Matrix3 {
            data: [
                [self.data[0][0], self.data[1][0], self.data[2][0]],
                [self.data[0][1], self.data[1][1], self.data[2][1]],
                [self.data[0][2], self.data[1][2], self.data[2][2]],
            ],
        }
    }

    /// Transform a vector by this matrix
    pub fn transform_vector(&self, v: Vector3) -> Vector3 {
        Vector3 {
            data: [
                self.data[0][0] * v.data[0] + self.data[0][1] * v.data[1] + self.data[0][2] * v.data[2],
                self.data[1][0] * v.data[0] + self.data[1][1] * v.data[1] + self.data[1][2] * v.data[2],
                self.data[2][0] * v.data[0] + self.data[2][1] * v.data[1] + self.data[2][2] * v.data[2],
            ],
        }
    }

    /// Access element at row, col (0-indexed)
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row][col]
    }

    /// Set element at row, col (0-indexed)
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row][col] = value;
    }

    /// Get determinant of matrix
    pub fn determinant(&self) -> f64 {
        let a = self.data[0][0];
        let b = self.data[0][1];
        let c = self.data[0][2];
        let d = self.data[1][0];
        let e = self.data[1][1];
        let f = self.data[1][2];
        let g = self.data[2][0];
        let h = self.data[2][1];
        let i = self.data[2][2];

        a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    }
}

impl Vector3 {
    /// Create new vector from array
    pub fn new(data: [f64; 3]) -> Self {
        Vector3 { data }
    }

    /// Create zero vector
    pub fn zero() -> Self {
        Vector3 { data: [0.0, 0.0, 0.0] }
    }

    /// Get magnitude (length) of vector
    pub fn magnitude(&self) -> f64 {
        (self.data[0].powi(2) + self.data[1].powi(2) + self.data[2].powi(2)).sqrt()
    }

    /// Get unit vector in same direction
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            *self
        } else {
            Vector3 {
                data: [self.data[0] / mag, self.data[1] / mag, self.data[2] / mag],
            }
        }
    }

    /// Dot product with another vector
    pub fn dot(&self, other: &Vector3) -> f64 {
        self.data[0] * other.data[0] + self.data[1] * other.data[1] + self.data[2] * other.data[2]
    }

    /// Cross product with another vector
    pub fn cross(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            data: [
                self.data[1] * other.data[2] - self.data[2] * other.data[1],
                self.data[2] * other.data[0] - self.data[0] * other.data[2],
                self.data[0] * other.data[1] - self.data[1] * other.data[0],
            ],
        }
    }

    /// Access element at index
    pub fn get(&self, index: usize) -> f64 {
        self.data[index]
    }

    /// Set element at index
    pub fn set(&mut self, index: usize, value: f64) {
        self.data[index] = value;
    }
}

// Matrix multiplication
impl Mul<Matrix3> for Matrix3 {
    type Output = Matrix3;

    fn mul(self, rhs: Matrix3) -> Self::Output {
        let mut result = Matrix3::zero();
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result.data[i][j] += self.data[i][k] * rhs.data[k][j];
                }
            }
        }
        result
    }
}

// Matrix addition
impl Add<Matrix3> for Matrix3 {
    type Output = Matrix3;

    fn add(self, rhs: Matrix3) -> Self::Output {
        let mut result = Matrix3::zero();
        for i in 0..3 {
            for j in 0..3 {
                result.data[i][j] = self.data[i][j] + rhs.data[i][j];
            }
        }
        result
    }
}

// Vector addition
impl Add<Vector3> for Vector3 {
    type Output = Vector3;

    fn add(self, rhs: Vector3) -> Self::Output {
        Vector3 {
            data: [
                self.data[0] + rhs.data[0],
                self.data[1] + rhs.data[1],
                self.data[2] + rhs.data[2],
            ],
        }
    }
}

// Scalar multiplication for Vector3
impl Mul<f64> for Vector3 {
    type Output = Vector3;

    fn mul(self, scalar: f64) -> Self::Output {
        Vector3 {
            data: [
                self.data[0] * scalar,
                self.data[1] * scalar,
                self.data[2] * scalar,
            ],
        }
    }
}

// Conversion from array to Vector3
impl From<[f64; 3]> for Vector3 {
    fn from(array: [f64; 3]) -> Self {
        Vector3 { data: array }
    }
}

// Conversion from Vector3 to array
impl From<Vector3> for [f64; 3] {
    fn from(vector: Vector3) -> Self {
        vector.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_identity() {
        let id = Matrix3::identity();
        assert_eq!(id.data[0][0], 1.0);
        assert_eq!(id.data[1][1], 1.0);
        assert_eq!(id.data[2][2], 1.0);
        assert_eq!(id.data[0][1], 0.0);
    }

    #[test]
    fn test_matrix_multiplication() {
        let a = Matrix3::new([
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        ]);
        let b = Matrix3::identity();
        let result = a * b;
        assert_eq!(result, a);
    }

    #[test]
    fn test_rotation_matrices() {
        let angle = std::f64::consts::PI / 2.0; // 90 degrees
        
        // Test X rotation
        let rx = Matrix3::rotation_x(angle);
        let v = Vector3::new([0.0, 1.0, 0.0]);
        let rotated = rx.transform_vector(v);
        assert!((rotated.data[0] - 0.0).abs() < 1e-10);
        assert!((rotated.data[1] - 0.0).abs() < 1e-10);
        assert!((rotated.data[2] - 1.0).abs() < 1e-10);

        // Test Z rotation 
        let rz = Matrix3::rotation_z(angle);
        let v2 = Vector3::new([1.0, 0.0, 0.0]);
        let rotated2 = rz.transform_vector(v2);
        assert!((rotated2.data[0] - 0.0).abs() < 1e-10);
        assert!((rotated2.data[1] - 1.0).abs() < 1e-10);
        assert!((rotated2.data[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_vector_operations() {
        let v1 = Vector3::new([1.0, 2.0, 3.0]);
        let v2 = Vector3::new([4.0, 5.0, 6.0]);
        
        // Test dot product
        let dot = v1.dot(&v2);
        assert_eq!(dot, 32.0); // 1*4 + 2*5 + 3*6
        
        // Test magnitude
        let mag = v1.magnitude();
        assert!((mag - (14.0_f64).sqrt()).abs() < 1e-10);
        
        // Test cross product
        let cross = v1.cross(&v2);
        assert_eq!(cross.data[0], -3.0); // 2*6 - 3*5
        assert_eq!(cross.data[1], 6.0);  // 3*4 - 1*6
        assert_eq!(cross.data[2], -3.0); // 1*5 - 2*4
    }

    #[test]
    fn test_matrix_transpose() {
        let m = Matrix3::new([
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        ]);
        let mt = m.transpose();
        assert_eq!(mt.data[0][1], 4.0);
        assert_eq!(mt.data[1][0], 2.0);
        assert_eq!(mt.data[2][1], 6.0);
    }

    #[test]
    fn test_matrix_determinant() {
        let m = Matrix3::new([
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 4.0],
            [5.0, 6.0, 0.0],
        ]);
        let det = m.determinant();
        assert_eq!(det, 1.0); // Calculated determinant
    }
}