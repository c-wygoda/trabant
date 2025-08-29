---
name: propagator
status: backlog
created: 2025-08-29T14:31:14Z
progress: 0%
prd: .claude/prds/propagator.md
github: [Will be updated when synced to GitHub]
---

# Epic: Propagator

## Overview
Implementation of a high-precision SGP4 satellite propagator in pure Rust with coordinate frame transformations (TEME → GCRS → ITRS) and optimized pass prediction. The system must match skyfield's accuracy (< 0.001° deviation) while providing sub-50ms performance for week-long pass predictions.

## Architecture Decisions

### Core Design Choices
- **Pure Rust SGP4**: Implement SGP4 algorithm from first principles using f64 precision throughout
- **Fixture-Driven Development**: Use existing skyfield-generated fixtures as golden standard for validation
- **Modular Architecture**: Separate concerns into distinct modules (sgp4, coordinates, eop, passes)
- **Zero-Copy EOP**: Parse and cache EOP data once, use references throughout

### Technology Stack
- **Math**: Use existing Rust math primitives, avoid external linear algebra libraries initially
- **Time**: chrono for date/time handling with leap second support
- **Serialization**: serde + serde_json for fixture parsing and output
- **WASM**: wasm-bindgen for browser integration (deferred to later phase)

### Performance Strategy
- **EOP Caching**: Parse CSV once, build interpolation lookup table
- **Pass Prediction**: Binary search for rise/set events, Newton-Raphson for refinement
- **Coordinate Transforms**: Pre-compute transformation matrices where possible

## Technical Approach

### Core Modules

#### SGP4 Engine (`src/sgp4/`)
- TLE/OMM parser with validation
- SGP4 propagator implementation per Vallado
- TEME position/velocity output
- Edge case handling (decay, hyperbolic)

#### Coordinate Systems (`src/coordinates/`)
- TEME → GCRS transformation (precession, nutation)
- GCRS → ITRS transformation (Earth rotation, polar motion)
- Transformation matrix caching
- Vector/matrix operations

#### EOP Data Handler (`src/eop/`)
- CSV parser for IERS EOP data
- Linear interpolation between values
- Error on out-of-range dates
- Memory-efficient storage

#### Pass Predictor (`src/passes/`)
- Observer location handling (geodetic → ECEF)
- Elevation/azimuth calculations
- Binary search for horizon crossings
- Maximum elevation finder

### Data Flow
1. Parse TLE/OMM → Internal orbital elements
2. Propagate with SGP4 → TEME coordinates
3. Transform TEME → GCRS → ITRS (if needed)
4. Calculate topocentric view → Az/El for observer
5. Find pass events → Rise/Max/Set times

## Implementation Strategy

### Development Phases
1. **Foundation**: Core SGP4 algorithm with fixture validation
2. **Coordinates**: TEME/GCRS/ITRS transformations with EOP
3. **Predictions**: Pass prediction with observer calculations
4. **Optimization**: Performance tuning and WASM preparation

### Testing Approach
- Unit tests for each mathematical function
- Integration tests against all fixture files
- Property-based testing for edge cases
- Benchmark tests for performance validation

### Risk Mitigation
- Start with simplified EOP (no interpolation) to validate transforms
- Use fixture data incrementally (positions → passes)
- Profile early to identify bottlenecks
- Keep WASM concerns separate from core logic

## Task Breakdown Preview

Simplified task structure focusing on incremental delivery:

- [ ] **Task 1: TLE/OMM Parser** - Parse orbital elements from TLE and OMM formats with validation
- [ ] **Task 2: Core SGP4 Implementation** - Implement SGP4 algorithm matching skyfield TEME positions
- [ ] **Task 3: EOP Data Handler** - Parse and interpolate Earth orientation parameters from CSV
- [ ] **Task 4: Coordinate Transformations** - TEME → GCRS → ITRS transformations with EOP data
- [ ] **Task 5: Observer Calculations** - Geodetic conversions and topocentric Az/El calculations
- [ ] **Task 6: Pass Prediction Engine** - Find rise/max/set events with binary search
- [ ] **Task 7: Integration Testing** - Validate against all fixture files with < 0.001° accuracy
- [ ] **Task 8: Performance Optimization** - Achieve < 50ms for 7-day predictions

## Dependencies

### External Libraries (Minimal)
- chrono: Date/time handling with leap seconds
- serde/serde_json: JSON parsing for fixtures and output
- csv: EOP data parsing (or manual implementation)

### Data Dependencies
- `tests/fixtures/eop.csv`: Earth orientation parameters
- `tests/fixtures/hotsat1-*.json`: Validation fixtures
- TLE/OMM orbital element sources

### Development Dependencies
- Python + skyfield: Fixture generation
- cargo bench: Performance validation
- wasm-pack: WASM compilation (later phase)

## Success Criteria (Technical)

### Accuracy Gates
- Position error < 10 meters vs skyfield
- Pass timing error < 1 second
- Az/El error < 0.001°
- 100% fixture test passage

### Performance Benchmarks
- Single propagation < 1 microsecond
- 7-day pass prediction < 50ms
- Memory usage < 10MB
- WASM size < 500KB (future)

### Code Quality
- No unsafe code in core algorithms
- > 95% test coverage
- Zero panics in production paths
- Deterministic results

## Estimated Effort

### Timeline
- **Week 1-2**: Core SGP4 + basic testing
- **Week 2-3**: Coordinate systems + EOP
- **Week 3-4**: Pass prediction + optimization
- **Week 4-5**: Polish + documentation

### Critical Path
1. SGP4 accuracy (blocks everything)
2. EOP data handling (blocks ITRS)
3. Pass prediction (blocks delivery)

### Resource Requirements
- Domain expertise for SGP4 validation
- Fixture data from skyfield
- Test satellite TLEs

## Tasks Created
- [ ] 001.md - TLE/OMM Parser (parallel: true)
- [ ] 002.md - Core SGP4 Implementation (parallel: false)
- [ ] 003.md - EOP Data Handler (parallel: true)
- [ ] 004.md - Coordinate Transformations (parallel: false)
- [ ] 005.md - Observer Calculations (parallel: false)
- [ ] 006.md - Pass Prediction Engine (parallel: false)
- [ ] 007.md - Integration Testing (parallel: false)
- [ ] 008.md - Performance Optimization (parallel: false)

Total tasks: 8
Parallel tasks: 2
Sequential tasks: 6
Estimated total effort: 84-116 hours