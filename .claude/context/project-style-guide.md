---
created: 2025-08-29T13:56:36Z
last_updated: 2025-08-29T13:56:36Z
version: 1.0
author: Claude Code PM System
---

# Project Style Guide

## Code Conventions

### Rust Style Standards
- **Edition**: Use Rust 2024 edition features and idioms
- **Formatting**: Follow standard `rustfmt` conventions
- **Naming**: Standard Rust naming conventions (snake_case for functions, PascalCase for types)
- **Documentation**: Comprehensive doc comments for public interfaces

### File Organization
- **Module Structure**: Logical separation by functional areas
- **File Names**: Descriptive, lowercase with underscores (e.g., `orbital_mechanics.rs`)
- **Directory Layout**: Standard Rust project structure with clear module hierarchy

## Development Patterns

### Script Development
- **Location**: All convenience scripts in `scripts/` directory
- **Shell Scripts**: Use `#!/bin/sh` for maximum compatibility
- **Python Scripts**: Use `uv` with inline dependency management
- **Naming**: Descriptive names indicating purpose (e.g., `generate-fixtures.py`)

### Error Handling
- **Fail Fast**: Critical configuration errors should terminate early
- **Graceful Degradation**: Optional features should log and continue
- **User-Friendly Messages**: Clear error reporting through resilience layer

## Testing Standards

### Test Structure
- **Comprehensive Coverage**: Implement tests for every function
- **Verbose Output**: Design tests to aid debugging with detailed output
- **Real Usage Patterns**: Tests must reflect actual usage scenarios
- **No Mock Services**: Use real implementations and fixtures

### Validation Approach
- **Reference Accuracy**: Validate against Python skyfield calculations
- **Precision Testing**: Numerical accuracy within defined tolerances
- **Edge Case Coverage**: Test boundary conditions and error scenarios

## Code Quality Rules

### Implementation Standards
- **No Partial Implementation**: Complete all features fully
- **No Simplification Comments**: Avoid "simplified for now" placeholders
- **No Code Duplication**: Reuse existing functions and constants
- **No Dead Code**: Remove unused code completely

### Architecture Principles
- **No Over-Engineering**: Prefer simple solutions over complex patterns
- **Separation of Concerns**: Keep validation, business logic, and interfaces separate
- **Resource Management**: Properly handle connections, timeouts, and cleanup

## Documentation Style

### Code Comments
- **Public APIs**: Comprehensive doc comments with examples
- **Complex Logic**: Explain the "why" behind non-obvious implementations
- **Mathematical Functions**: Document algorithms and precision expectations
- **WASM Interfaces**: Clear documentation for JavaScript integration

### File Headers
- **Purpose**: Brief description of file's responsibility
- **Dependencies**: Note external dependencies and their usage
- **Precision Notes**: Document accuracy expectations for calculations

## Naming Conventions

### Functions and Variables
- **Descriptive Names**: Clear indication of purpose and behavior
- **Consistent Patterns**: Follow established patterns in existing code
- **Astronomical Terms**: Use standard astronomical terminology accurately
- **Units**: Include units in variable names where relevant (e.g., `elevation_degrees`)

### Constants and Configuration
- **All Caps**: Standard Rust convention for constants
- **Grouped Logically**: Related constants organized together
- **Well Documented**: Clear explanation of values and their significance

## Commit and Version Control

### Commit Standards
- **Conventional Commits**: Follow conventional commit message format
- **Atomic Changes**: One logical change per commit
- **Test Requirements**: All commits must pass existing tests
- **Documentation Updates**: Update docs with code changes

### Branch Strategy
- **Main Branch**: Stable, deployable code only
- **Feature Branches**: Isolated development of new capabilities
- **Testing**: Comprehensive validation before merge to main