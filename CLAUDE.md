# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Building and Testing
- Build: `cargo build`
- Run tests: `cargo test`
- Bootstrap development environment: `scripts/bootstrap.sh` (installs wasm-pack to local/bin)
- Generate test fixtures: `scripts/generate-fixtures.py` (creates reference data using Python's skyfield)

### Development Environment Setup
- Uses local toolchain in `local/bin/` directory
- wasm-pack is installed locally via bootstrap script

## Architecture

This is a Rust satellite tracking and pass prediction system called "Trabant". The project structure:

- `src/lib.rs` - Main library code (currently minimal stub)
- `scripts/` - Development and utility scripts
  - `bootstrap.sh` - Sets up local wasm-pack installation
  - `generate-fixtures.py` - Python script using skyfield to generate reference benchmark data
- `tests/fixtures/` - JSON test data for validation
  - `hotsat1-omm.json` - OMM (Orbital Mean-elements Message) data for HOTSAT-1 satellite
  - `hotsat1-berlin-passes.json` - Reference pass predictions for Berlin location
  - `hotsat1-positions.json` - Reference position calculations

The system generates satellite pass predictions and validates against Python skyfield calculations. The fixture generator creates benchmark data for satellite passes over Berlin (52.52°N, 13.405°E) with minimum 15° elevation, position calculations in TEME and GCRS coordinate frames, and pass timing/elevation data.

## Sub-Agent Usage

- Use `file-analyzer` agent when asked to read files for concise summaries
- Use `code-analyzer` agent for code analysis, bug research, or logic tracing  
- Use `test-runner` agent to execute tests and analyze results

## Development Rules

- Convenience scripts must be kept in `scripts/` folder using shell or Python scripts with `uv`
- Test against fixture files generated using Python's skyfield package for precision validation
- No partial implementations or simplifications
- No code duplication - check existing codebase and reuse functions
- Implement tests for every function with verbose output for debugging
- Follow existing naming patterns and conventions
- Always run tests before committing
- Use conventional commits style for commit messages
