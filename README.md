# Waveform MCP Server

An MCP (Model Context Protocol) server for reading and analyzing waveform files (VCD/FST format) using the [wellen](https://github.com/ekiwi/wellen) library.

## Features

- Open VCD (Value Change Dump) and FST (Fast Signal Trace) waveform files
- List all signals in a waveform with hierarchical paths
- Read signal values at specific time indices (single or multiple)
- Get signal metadata (type, width, index range)
- Find signal events (changes) within a time range
- Format time values with timescale information (e.g., "10ns", "5000ps")

## Tools

The server provides 5 MCP tools:

1. **open_waveform** - Open a waveform file
   - `file_path`: Path to .vcd or .fst file
   - `alias`: Optional alias for the waveform (defaults to filename)

2. **list_signals** - List all signals in an open waveform
   - `waveform_id`: ID or alias of the waveform
   - `name_pattern`: Optional substring to filter signals by name (case-insensitive)
   - `hierarchy_prefix`: Optional prefix to filter signals by hierarchy path
   - `limit`: Optional maximum number of signals to return

3. **read_signal** - Read signal values at specific time indices
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal (e.g., "top.module.signal")
   - `time_index`: Optional single time index to read
   - `time_indices`: Optional array of time indices to read multiple values

4. **get_signal_info** - Get metadata about a signal
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal
   - Returns: Signal type, width (in bits), and index range (e.g., "[7:0]")

5. **find_signal_events** - Find all signal changes within a time range
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal
   - `start_time_index`: Optional start of time range (default: 0)
   - `end_time_index`: Optional end of time range (default: last time index)
   - `limit`: Optional maximum number of events to return (default: unlimited)

## Installation

```bash
# Clone the repository
git clone https://github.com/jiegec/waveform-mcp
cd waveform-mcp

# Build the server
cargo build --release
```

The built binary will be at `target/release/waveform-mcp`. It uses STDIO for transport. Configure your MCP client accordingly.

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

### Running

```bash
# Run the server directly (for testing with stdio)
cargo run
```

The server uses stdio transport for MCP communication.

## License

[MIT](LICENSE)
