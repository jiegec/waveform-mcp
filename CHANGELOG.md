# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `list_signals` supports glob (`match_mode: "glob"`, CLI `--glob`) and regular expression
  (`match_mode: "regex"`, CLI `--regex`) signal path matching
- Multiple `list_signals` patterns of the same mode can be given at once; they are OR-ed together
- A hint is printed when a substring pattern contains glob wildcards but matched nothing

### Added
- Not-found errors now list the available names, and suggest the closest match for typos
  (waveform ids today; signals and scopes follow)
- `waveform-cli --help`/`-h`, `waveform-cli help <command>` and `waveform-cli <command> --help`
  print general or per-command help; `--version`/`-V` prints the version

### Changed
- CLI `-h` now means help; the `list_signals` scope filter is `--hierarchy` only
- The `list_signals` MCP parameter `name_pattern` was renamed to `pattern` (`name_pattern`
  is still accepted as an alias)
- `list_signals` now reports invalid glob/regex patterns as errors instead of returning nothing
- CLI `list_signals`: the match mode is selected by `--contains` / `--glob` / `--regex`
  (mutually exclusive) instead of a separate `--match-mode` flag

## [0.6.0] - 2026-08-01

### Changed
- Improved signal lookup performance
- Updated dependencies
- Four- and nine-state signal values are now rendered as Verilog-style ASCII bit strings (e.g. `4'b10xz`)

## [0.5.0] - 2026-05-04

### Added
- `read_hierarchy` tool for reading waveform scope trees from the MCP server and CLI

### Changed
- Bumped to Rust 2024 edition
- `read_hierarchy` now returns only module scopes

### Documentation
- Added cargo install instructions and reorganized README usage section
- Fixed installation path from `~/.local/bin` to `~/.cargo/bin`
- Added crates.io link and badge
- Removed too-long keyword from Cargo.toml

## [0.4.0] - 2026-03-11

### Added
- **Standalone CLI binary** (`waveform-cli`) - Direct command-line access to all waveform tools
  - All 7 tools available: `open_waveform`, `close_waveform`, `list_signals`, `read_signal`, `get_signal_info`, `find_signal_events`, `find_conditional_events`
  - Command chaining using `--` separator
  - State persistence across chained commands
  - Full help text and usage examples
- **CLI parser library module** (`src/cli_parser.rs`) with comprehensive unit tests
  - 18 test cases covering all CLI commands and error handling
  - Extracted from binary to enable testing
- **close_waveform** tool - Close waveforms and free memory
  - Useful for long-running HTTP sessions to manage memory usage
  - Returns success confirmation or error if waveform not found
- Server instructions updated to include `close_waveform`

### Changed
- Updated `rmcp` from 0.14 to 0.16
- Updated `lalrpop` and `lalrpop-util` from 0.22 to 0.23

### Documentation
- Added CLI section to README.md with usage examples
- Added CLI section to CLAUDE.md for developer notes

## [0.3.0] - 2025-12-28

### Added
- **Bitwise operations** to condition search language:
  - Bitwise AND (`&`), OR (`|`), XOR (`^`), and NOT (`~`) operators
  - Bit extraction support for accessing individual bits and slices
- **Streamable HTTP transport** support with CLI option (`--http` flag)
- Handshake cycle example to documentation

### Changed
- Migrated condition evaluation from `i64` to `BigUint` for supporting arbitrary-precision integer operations

### Fixed
- Signal name in `find_conditional_events` example
- Bitwise operator precedence to match documented precedence rules

### Documentation
- Added operator precedence rules to QWEN.md
- Mentioned bitwise operators in condition search documentation
- Added return value examples for MCP tools in README
- Added missing `recursive` parameter to `list_signals` documentation

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

[unreleased]: https://github.com/jiegec/waveform-mcp/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/jiegec/waveform-mcp/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/jiegec/waveform-mcp/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/jiegec/waveform-mcp/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/jiegec/waveform-mcp/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/jiegec/waveform-mcp/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jiegec/waveform-mcp/releases/tag/v0.1.0
