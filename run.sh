#!/bin/sh
cd "$(dirname "$0")"
cargo build >/dev/null 2>&1
./target/debug/waveform-mcp
