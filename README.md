# Waveform MCP Server

[![Crates.io Version](https://img.shields.io/crates/v/waveform-mcp)](https://crates.io/crates/waveform-mcp)

An MCP (Model Context Protocol) server for reading and analyzing waveform files (VCD/FST format) using the [wellen](https://github.com/ekiwi/wellen) library.

## Usage

### Installation

Install via cargo:

```bash
cargo install waveform-mcp
```

The built binary will be at `~/.cargo/bin/waveform-mcp`.

Install manually:

```bash
# Clone the repository
git clone https://github.com/jiegec/waveform-mcp
cd waveform-mcp

# Build the server
cargo build --release
```

The built binary will be at `target/release/waveform-mcp`.

### Running

```bash
# Run the server with stdio transport (default)
target/release/waveform-mcp

# Run the server in HTTP mode
target/release/waveform-mcp --http

# Run the server in HTTP mode with custom bind address
target/release/waveform-mcp --http --bind-address 0.0.0.0:8000
```

The server supports two transport modes:

- **Stdio mode** (default): Uses standard input/output for MCP communication
- **HTTP mode**: Uses streamable HTTP server for remote access at `/mcp` endpoint

When running in HTTP mode, the server listens on the specified bind address (default: `127.0.0.1:8000`). HTTP mode allows the waveform store to be shared across multiple HTTP sessions, enabling remote analysis of waveform files.

## Features

- Open VCD (Value Change Dump) and FST (Fast Signal Trace) waveform files
- List all signals in a waveform with hierarchical paths
- Read the waveform module hierarchy as an indented tree
- Read signal values at specific time indices (single or multiple)
- Get signal metadata (type, width, index range)
- Find signal events (changes) within a time range
- Format time values with timescale information (e.g., "10ns", "5000ps")
- Streamable HTTP server support for remote access

## Tools

The server provides 8 MCP tools:

1. **open_waveform** - Open a waveform file
   - `file_path`: Path to .vcd or .fst file
   - `alias`: Optional alias for the waveform (defaults to filename)

   **Example response:**
   ```
   Waveform opened successfully with alias: waveform.vcd
   ```

2. **close_waveform** - Close a waveform and free its memory
   - `waveform_id`: ID or alias of the waveform to close

   **Example response:**
   ```
   Waveform 'waveform.vcd' closed successfully
   ```

3. **list_signals** - List all signals in an open waveform
   - `waveform_id`: ID or alias of the waveform
   - `name_pattern`: Optional substring to filter signals by name (case-insensitive)
   - `hierarchy_prefix`: Optional prefix to filter signals by hierarchy path
   - `recursive`: Optional flag to include signals from sub-hierarchies (default: false)
   - `limit`: Optional maximum number of signals to return (default: 100)

   **Example response:**
   ```
   Found 3 signals:
   top.clock
   top.reset
   top.data
   ```

4. **read_hierarchy** - Read the waveform module hierarchy as an indented tree
   - `waveform_id`: ID or alias of the waveform
   - `scope_path`: Optional scope path to start from
   - `recursive`: Optional flag to include descendants (default: false)
   - `limit`: Optional maximum number of modules to return (default: 200)

   **Example response:**
   ```
   Hierarchy rooted at 'top':
   top
     submodule
   ```

5. **read_signal** - Read signal values at specific time indices
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal (e.g., "top.module.signal")
   - `time_index`: Optional single time index to read
   - `time_indices`: Optional array of time indices to read multiple values

   **Example response:**
   ```
   Time index 0 (0ns): 0
   Time index 10 (10ns): 1
   Time index 20 (20ns): 1
   ```

6. **get_signal_info** - Get metadata about a signal
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal

   **Example response:**
   ```
   Signal: top.data
   Type: Wire
   Width: 8 bits
   Index: [7:0]
   ```

7. **find_signal_events** - Find all signal changes within a time range
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal
   - `start_time_index`: Optional start of time range (default: 0)
   - `end_time_index`: Optional end of time range (default: last time index)
   - `limit`: Optional maximum number of events to return (default: unlimited)

   **Example response:**
   ```
   Found 3 events for signal 'top.clock' (time range: 0 to 20):
   Time index 0 (0ns): 0
   Time index 10 (10ns): 1
   Time index 20 (20ns): 0
   ```

8. **find_conditional_events** - Find events where a condition is satisfied
   - `waveform_id`: ID or alias of waveform
   - `condition`: Conditional expression to evaluate
   - `start_time_index`: Optional start of time range (default: 0)
   - `end_time_index`: Optional end of time range (default: last time index)
   - `limit`: Optional maximum number of events to return (default: 100)

   **Example response:**
   ```
   Found 2 events for condition '!$past(TOP.signal) && TOP.signal' (time range: 0 to 50):
   Time index 5 (50ns): top.signal = 8'h0A
   Time index 15 (150ns): top.signal = 8'hFF
   ```

   **Supported condition syntax:**
   - Signal paths (e.g., `TOP.signal`)
   - Bitwise operators: `~` (NOT), `&` (AND), `|` (OR), `^` (XOR)
   - Boolean operators: `&&` (AND), `||` (OR), `!` (NOT)
   - Comparison operators: `==`, `!=`
   - Parentheses for grouping: `(condition)`
   - `$past(signal)` - read signal value from previous time index
   - Verilog-style literals: `4'b0101` (binary), `3'd2` (decimal), `5'h1A` (hex)
   - Bit extraction: `signal[bit]` for single bit, `signal[msb:lsb]` for range

   **Operator precedence (highest to lowest):**
   1. `~`, `!` (bitwise NOT, logical NOT)
   2. `==`, `!=` (equality/inequality)
   3. `&` (bitwise AND)
   4. `^` (bitwise XOR)
   5. `|` (bitwise OR)
   6. `&&` (logical AND)
   7. `||` (logical OR)

   **Examples:**
   - Find when signal1 AND signal2 are true: `TOP.signal1 && TOP.signal2`
   - Find when counter equals a specific value: `TOP.counter == 4'd10`
   - Find rising edge: `!$past(TOP.signal) && TOP.signal`
   - Find falling edge: `$past(TOP.signal) && !TOP.signal`
   - Find handshake cycles (when both valid and ready are asserted): `TOP.valid && TOP.ready`
   - Complex condition: `(TOP.valid && TOP.data != 8'hFF) || TOP.error`
   - Bitwise operations: `TOP.flags & 4'b0001` (check if bit 0 is set)
   - Bitwise NOT: `~TOP.mask` (invert all bits)

## Standalone CLI

In addition to the MCP server, a standalone CLI tool `waveform-cli` is available for direct command-line access without an MCP client.

### CLI Usage

```bash
# Basic command
waveform-cli open_waveform /path/to/waveform.vcd

# Chain multiple commands with --
waveform-cli open_waveform test.vcd -- list_signals test.vcd --pattern clk

# Full workflow example
waveform-cli open_waveform test.vcd --alias mywave -- \\
  list_signals mywave -- \\
  read_signal mywave top.clk --time-indices 0,1,2,3 -- \\
  close_waveform mywave
```

### CLI Commands

- **open_waveform** `<file_path>` [--alias `<name>`] - Open waveform file
- **close_waveform** `<id>` - Close waveform
- **list_signals** `<id>` [--pattern `<p>`] [--hierarchy `<h>`] [--recursive `<bool>`] [--limit `<n>`]
- **read_hierarchy** `<id>` [--scope `<scope>`] [--recursive `<bool>`] [--limit `<n>`]
- **read_signal** `<id>` `<signal>` [--time-index `<idx>` | --time-indices `<list>`]
- **get_signal_info** `<id>` `<signal>` - Get signal metadata
- **find_signal_events** `<id>` `<signal>` [--start `<idx>`] [--end `<idx>`] [--limit `<n>`]
- **find_conditional_events** `<id>` `<condition>` [--start `<idx>`] [--end `<idx>`] [--limit `<n>`]


## Development

### Building

```bash
cargo build
cargo build --release
```

### Testing

```bash
cargo test
```
## License

[MIT](LICENSE)
