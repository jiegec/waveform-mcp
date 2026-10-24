//! Waveform MCP Server Library
//!
//! This library provides utilities for working with waveform files.

pub mod cli_parser;
pub mod condition;
pub mod formatting;
pub mod hierarchy;
pub mod matcher;
pub mod signal;
pub mod suggest;

// Re-export public functions
pub use cli_parser::{
    Command, SignalPattern, command_help, help_request, parse_args, split_command_groups,
    usage_text,
};
pub use condition::find_conditional_events;
pub use formatting::{format_signal_value, format_time};
pub use hierarchy::all_scope_paths;
pub use hierarchy::all_signal_paths;
pub use hierarchy::find_scope_by_path;
pub use hierarchy::find_signal_by_path;
pub use hierarchy::read_hierarchy;
pub use hierarchy::scope_not_found_message;
pub use hierarchy::signal_not_found_message;
pub use matcher::{MatchMode, SignalMatcher, glob_hint, pattern_seed};
pub use signal::find_signal_events;
pub use signal::format_signal_query_result;
pub use signal::get_signal_metadata;
pub use signal::list_signals;
pub use signal::read_signal_values;
pub use suggest::{did_you_mean, not_found_message};
