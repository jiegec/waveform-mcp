//! Waveform MCP Server Library
//!
//! This library provides utilities for working with waveform files.

/// Format a time value with its timescale into a human-readable string.
///
/// # Arguments
/// * `time_value` - The raw time value from the waveform
/// * `timescale` - Optional timescale information for proper formatting
///
/// # Examples
/// ```
/// use waveform_mcp::format_time;
///
/// let timescale = wellen::Timescale {
///     factor: 1,
///     unit: wellen::TimescaleUnit::NanoSeconds,
/// };
/// assert_eq!(format_time(10, Some(&timescale)), "10ns");
/// ```
pub fn format_time(time_value: u64, timescale: Option<&wellen::Timescale>) -> String {
    match timescale {
        Some(ts) => {
            let unit = match ts.unit {
                wellen::TimescaleUnit::ZeptoSeconds => "zs",
                wellen::TimescaleUnit::AttoSeconds => "as",
                wellen::TimescaleUnit::FemtoSeconds => "fs",
                wellen::TimescaleUnit::PicoSeconds => "ps",
                wellen::TimescaleUnit::NanoSeconds => "ns",
                wellen::TimescaleUnit::MicroSeconds => "us",
                wellen::TimescaleUnit::MilliSeconds => "ms",
                wellen::TimescaleUnit::Seconds => "s",
                wellen::TimescaleUnit::Unknown => "unknown",
            };
            format!("{}{}", time_value * ts.factor as u64, unit)
        }
        None => format!("{} (unknown timescale)", time_value),
    }
}

/// Format a signal value into a human-readable string.
///
/// # Arguments
/// * `signal_value` - The signal value to format
///
/// # Returns
/// A string representation of the signal value.
pub fn format_signal_value(signal_value: wellen::SignalValue) -> String {
    match signal_value {
        wellen::SignalValue::Event => "Event".to_string(),
        wellen::SignalValue::Binary(data, _) => format!("{:?}", data),
        wellen::SignalValue::FourValue(data, _) => format!("{:?}", data),
        wellen::SignalValue::NineValue(data, _) => format!("{:?}", data),
        wellen::SignalValue::String(s) => s.to_string(),
        wellen::SignalValue::Real(r) => format!("{}", r),
    }
}

/// Find a signal by its hierarchical path in the waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to the signal (e.g., "top.module.signal")
///
/// # Returns
/// `Some(SignalRef)` if the signal is found, `None` otherwise.
pub fn find_signal_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::SignalRef> {
    for var in hierarchy.iter_vars() {
        let signal_path = var.full_name(hierarchy);
        if signal_path == path {
            return Some(var.signal_ref());
        }
    }
    None
}

/// Find a scope by its hierarchical path in the waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to the scope (e.g., "top.module")
///
/// # Returns
/// `Some(ScopeRef)` if the scope is found, `None` otherwise.
pub fn find_scope_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::ScopeRef> {
    for scope_ref in hierarchy.scopes() {
        let scope = &hierarchy[scope_ref];
        let scope_path = scope.full_name(hierarchy);
        if scope_path == path {
            return Some(scope_ref);
        }
        // Recursively check child scopes
        if let Some(child_ref) = find_scope_by_path_recursive(hierarchy, scope_ref, path) {
            return Some(child_ref);
        }
    }
    None
}

fn find_scope_by_path_recursive(
    hierarchy: &wellen::Hierarchy,
    parent_ref: wellen::ScopeRef,
    target_path: &str,
) -> Option<wellen::ScopeRef> {
    let parent = &hierarchy[parent_ref];
    for child_ref in parent.scopes(hierarchy) {
        let child = &hierarchy[child_ref];
        let child_path = child.full_name(hierarchy);
        if child_path == target_path {
            return Some(child_ref);
        }
        // Recursively check child scopes
        if let Some(found) = find_scope_by_path_recursive(hierarchy, child_ref, target_path) {
            return Some(found);
        }
    }
    None
}

