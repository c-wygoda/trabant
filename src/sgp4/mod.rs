//! SGP4 orbital propagation and element parsing
//!
//! This module provides functionality for parsing and working with orbital elements
//! in various formats including Two-Line Element (TLE) sets and Orbital Mean-elements
//! Message (OMM) format.

pub mod elements;
pub mod tle;
pub mod omm;

pub use elements::OrbitalElements;
pub use tle::{TleParser, TleError};
pub use omm::{OmmParser, OmmError};