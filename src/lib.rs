//! Waveform MCP Server Library
//!
//! This library provides utilities for working with waveform files.

pub mod condition;
pub mod formatting;
pub mod hierarchy;
pub mod signal;

// Re-export public functions
pub use condition::find_conditional_events;
pub use formatting::{format_signal_value, format_time};
pub use hierarchy::find_scope_by_path;
pub use hierarchy::find_signal_by_path;
pub use signal::find_signal_events;
pub use signal::get_signal_metadata;
pub use signal::list_signals;
pub use signal::read_signal_values;
