# Waveform MCP Server

An MCP (Model Context Protocol) server for reading and analyzing waveform files (VCD/FST format) using the [wellen](https://github.com/ekiwi/wellen) library.

## Features

- Open VCD (Value Change Dump) and FST (Fast Signal Trace) waveform files
- List all signals in a waveform with hierarchical paths
- Read signal values at specific time indices
- Get signal metadata (type, width, encoding)

## Tools

The server provides 4 MCP tools:

1. **open_waveform** - Open a waveform file
   - `file_path`: Path to .vcd or .fst file
   - `alias`: Optional alias for the waveform (defaults to filename)

2. **list_signals** - List all signals in an open waveform
   - `waveform_id`: ID or alias of the waveform

3. **read_signal** - Read signal values at a specific time index
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal (e.g., "top.module.signal")
   - `time_index`: Time index to read the value at

4. **get_signal_info** - Get metadata about a signal
   - `waveform_id`: ID or alias of the waveform
   - `signal_path`: Hierarchical path to signal

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd waveform-mcp

# Build the server
cargo build --release
```

The built binary will be at `target/release/waveform-mcp`.

## Usage with MCP Clients

### Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS or `%APPDATA%/Claude/claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "waveform": {
      "command": "/path/to/waveform-mcp/target/release/waveform-mcp"
    }
  }
}
```

### Manual Testing

You can test the server manually using the provided example client:

```bash
# Build the server first
cargo build

# Run the example client
python3 example_client.py
```

The example client demonstrates the complete MCP protocol flow:
1. Initialize the connection
2. List available tools
3. Call a tool (open_waveform with dummy file)

## Protocol Flow

The MCP server follows the standard Model Context Protocol:

1. Client sends `initialize` request
2. Server responds with capabilities (including tools)
3. Client sends `initialized` notification
4. Client can now call `tools/list` and `tools/call`

Example initialization sequence:
```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"implementation": {"name": "test-client", "version": "1.0.0"}, "capabilities": {}}}
{"jsonrpc": "2.0", "method": "initialized", "params": {}}
{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
```

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

## Dependencies

- [wellen](https://crates.io/crates/wellen) - Waveform file parsing (VCD/FST)
- [mcp-sdk-rs](https://crates.io/crates/mcp-sdk-rs) - MCP server implementation
- [tokio](https://crates.io/crates/tokio) - Async runtime
- [serde](https://crates.io/crates/serde) - Serialization

## License

[MIT](LICENSE)
