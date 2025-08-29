---
name: propagator
description: High-precision SGP4 satellite propagator with ITRS coordinate conversions and pass prediction capabilities
status: backlog
created: 2025-08-29T14:20:24Z
---

# PRD: Propagator

## Executive Summary

The Trabant propagator is a high-precision satellite orbit propagation system implementing the SGP4 algorithm with comprehensive coordinate frame transformations and satellite pass prediction capabilities. The system prioritizes accuracy matching or exceeding Python's skyfield library while maintaining high performance for real-time applications and batch pass predictions.

## Problem Statement

Satellite tracking and pass prediction require complex orbital mechanics calculations with multiple coordinate system transformations. Current solutions often sacrifice either accuracy or performance. We need a Rust-based propagator that:
- Matches the precision of established libraries like skyfield (sub-meter accuracy)
- Provides fast pass prediction for ground station planning
- Supports modern web deployments via WASM
- Handles coordinate transformations with proper Earth orientation parameters

This is critical for applications requiring reliable satellite visibility predictions, including ground station scheduling, observation planning, and satellite operations.

## User Stories

### Primary User: Satellite Operations Engineer
**Story**: As a satellite operations engineer, I want to predict when satellites will be visible from my ground station so that I can schedule communication windows.

**Acceptance Criteria**:
- Predictions match skyfield accuracy within 0.001° for azimuth/elevation
- Pass timing accurate to within 1 second
- Can generate predictions for weeks ahead in under 100ms
- Supports minimum elevation constraints

### Secondary User: Web Application Developer
**Story**: As a web developer, I want to integrate satellite tracking into my browser-based application without server dependencies.

**Acceptance Criteria**:
- WASM module under 500KB
- JavaScript/TypeScript bindings available
- Can propagate single satellite in real-time (60fps)
- Memory-efficient for mobile browsers

### Tertiary User: Research Scientist
**Story**: As a researcher, I need precise satellite positions in multiple coordinate frames for data correlation and analysis.

**Acceptance Criteria**:
- Supports TEME, GCRS, and ITRS coordinate frames
- Position accuracy within 10 meters compared to skyfield
- Velocity vectors available for all frames
- Reproducible results with same input data

## Requirements

### Functional Requirements

#### Core Propagation Engine
- **SGP4 Implementation**
  - Full SGP4 algorithm (no SDP4 required)
  - Support for standard TLE format input
  - Support for OMM (Orbital Mean-elements Message) format
  - Handle edge cases (decayed orbits, hyperbolic orbits)

- **Coordinate Systems**
  - TEME (True Equator Mean Equinox) - native SGP4 output
  - GCRS (Geocentric Celestial Reference System)
  - ITRS (International Terrestrial Reference System)
  - Proper transformation matrices between all frames

- **Earth Orientation Parameters (EOP)**
  - Parse IERS EOP data (CSV format initially; fixture file provided in tests/fixtures/eop.csv)
  - Interpolation for dates between published values
  - Error for dates beyond EOP data range
  - Caching mechanism for performance

#### Pass Prediction System
- **Ground Station Visibility**
  - Calculate satellite passes for given observer location
  - Support minimum elevation constraints
  - Provide rise/culmination/set times
  - Calculate maximum elevation and azimuth

- **Prediction Performance**
  - Batch prediction for extended time periods
  - Early termination for efficiency
  - Configurable time resolution

- **Event Detection**
  - Precise rise/set time determination
  - Maximum elevation point calculation
  - Optional eclipse detection (future enhancement)

#### Data Structures
- **Input Formats**
  - TLE parser with validation
  - OMM/XML parser
  - Observer location (latitude, longitude, altitude)

- **Output Formats**
  - Position/velocity vectors
  - Pass prediction events
  - JSON serialization for all outputs

### Non-Functional Requirements

#### Performance
- Single satellite propagation: < 1 microsecond per time point
- Pass prediction for 7 days: < 50ms
- Memory footprint: < 10MB for single satellite
- WASM binary size: < 500KB

#### Accuracy
- Position error vs skyfield: < 10 meters
- Pass time error: < 1 second
- Azimuth/elevation error: < 0.001°
- Must pass all fixture validation tests

#### Reliability
- Deterministic results (same input = same output)
- Graceful handling of edge cases
- No panics in production code
- Comprehensive error messages

#### Compatibility
- Pure Rust implementation (no C dependencies)
- WASM compilation support
- No unsafe code in core algorithms
- Platform-agnostic (Windows, macOS, Linux)

## Success Criteria

### Quantitative Metrics
- **Accuracy**: 100% of fixture tests pass with < 0.001° deviation
- **Performance**: 1000 passes/second prediction rate
- **Size**: WASM module < 500KB gzipped
- **Coverage**: > 95% test coverage

### Qualitative Metrics
- Code passes review by domain expert
- Documentation sufficient for new developers
- API intuitive for common use cases
- Performance comparable to C++ implementations

## Constraints & Assumptions

### Constraints
- Must use Rust (no C/C++ dependencies)
- Cannot modify fixture data (golden standard)
- Single satellite at a time (parallelization out of scope)
- CSV format for EOP data (parquet format future enhancement)

### Assumptions
- SGP4 sufficient for required accuracy (no need for SGP8)
- Earth is WGS84 ellipsoid for ground calculations
- No atmospheric refraction corrections needed
- UTC time system throughout (no GPS/TAI conversions)
- EOP data available and regularly updated

## Out of Scope

The following features are explicitly NOT part of this phase:

- **Multi-satellite propagation**: Parallelization via web workers is future work
- **SDP4 algorithm**: Deep space propagation not required
- **Advanced perturbations**: Solar radiation pressure, third-body effects
- **Orbit determination**: No fitting of orbital elements
- **Collision detection**: No conjunction analysis
- **Visualization**: No rendering or graphics components
- **Binary EOP format**: Parquet support is future enhancement
- **Ground track calculation**: Not required for initial release
- **Atmospheric models**: No density calculations
- **GPS/GLONASS time systems**: UTC only

## Dependencies

### External Dependencies
- **IERS EOP Data**: Regular updates from IERS servers
- **Test Fixtures**: Skyfield-generated reference data
- **Rust Toolchain**: Latest stable Rust compiler
- **wasm-pack**: For WASM compilation

### Internal Dependencies
- **Mathematical Libraries**:
  - Linear algebra operations
  - Trigonometric functions with high precision
  - Date/time handling with leap seconds

- **Development Tools**:
  - Python + skyfield for fixture generation
  - Bootstrap script for environment setup
  - Test runner for validation

### Data Dependencies
- Valid TLE/OMM data for satellites
- Current EOP parameters file
- Observer location database (optional)

## Risk Mitigation

### Technical Risks
- **Numerical Precision**: Use f64 throughout, validate against fixtures
- **Coordinate Transform Complexity**: Extensive testing against skyfield
- **EOP Data Availability**: Implement fallback for missing data
- **WASM Performance**: Profile and optimize critical paths

### Schedule Risks
- **Algorithm Complexity**: Start with basic SGP4, iterate on accuracy
- **Testing Coverage**: Automate fixture generation and validation
- **Documentation**: Write docs alongside implementation

## Appendix

### Reference Materials
- Revisiting Spacetrack Report #3 (Vallado et al.)
- IERS Conventions (2010)
- Skyfield documentation and source code
- NORAD SGP4 test cases

### Glossary
- **EOP**: Earth Orientation Parameters
- **GCRS**: Geocentric Celestial Reference System
- **IERS**: International Earth Rotation Service
- **ITRS**: International Terrestrial Reference System
- **OMM**: Orbital Mean-elements Message
- **SGP4**: Simplified General Perturbations 4
- **TEME**: True Equator Mean Equinox
- **TLE**: Two-Line Element set
- **WASM**: WebAssembly
