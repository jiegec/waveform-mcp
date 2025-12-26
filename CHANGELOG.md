# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-12-26

### Added
- **Conditional event search** (`find_conditional_events`) - Find events where complex conditions are satisfied
  - Support for boolean operators: AND (`&&`), OR (`||`), NOT (`!`)
  - Support for comparison operators: equals (`==`), not-equals (`!=`)
  - Support for `$past(signal)` to access signal values from previous time index
  - Support for Verilog-style literals: binary (`4'b0101`), decimal (`3'd2`), hexadecimal (`5'h1A`)
  - Parentheses for expression grouping
  - Examples: `TOP.signal1 && TOP.signal2`, `TOP.counter == 4'd10`, `!$past(TOP.signal) && TOP.signal`

### Changed
- Replaced manual condition parser with **lalrpop** parser generator
  - Improved parsing reliability with formal grammar
  - Better error messages for malformed conditions
  - Reduced code complexity by removing ~300 lines of manual parsing code

### Internal
- Refactored codebase into modules:
  - `src/formatting.rs` - Time and value formatting functions
  - `src/hierarchy.rs` - Hierarchy navigation functions
  - `src/signal.rs` - Signal operations
  - `src/condition.rs` - Condition parsing and evaluation
  - Split test files by functionality (5 test files from single integration test)
- Simplified condition evaluation: merged boolean and numeric evaluation into single i64-based function

## [0.1.0] - Initial Release

### Added
- Basic waveform file support (VCD and FST formats)
- Open waveforms with optional aliases
- List all signals in hierarchical structure
- Read signal values at specific time indices (single or multiple)
- Get signal metadata (type, width, index range)
- Find signal events (changes) within time ranges
- Format time values with timescale information

### Tools
- `open_waveform` - Open a waveform file
- `list_signals` - List all signals with optional filtering
- `read_signal` - Read signal values
- `get_signal_info` - Get signal metadata
- `find_signal_events` - Find signal changes in time range
