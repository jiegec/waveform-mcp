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

/// Collect signals from a scope and optionally its children recursively.
fn collect_signals_from_scope(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    recursive: bool,
    name_pattern: Option<&str>,
) -> Vec<String> {
    let mut signals = Vec::new();
    let scope = &hierarchy[scope_ref];

    // Collect variables directly in this scope
    for var_ref in scope.vars(hierarchy) {
        let var = &hierarchy[var_ref];
        let path = var.full_name(hierarchy);

        // Apply name pattern filter if provided
        if let Some(pattern) = name_pattern {
            let pattern_lower = pattern.to_lowercase();
            let path_lower = path.to_lowercase();
            if !path_lower.contains(&pattern_lower) {
                continue;
            }
        }

        signals.push(path);
    }

    // If recursive, also collect from child scopes
    if recursive {
        for child_ref in scope.scopes(hierarchy) {
            signals.extend(collect_signals_from_scope(
                hierarchy,
                child_ref,
                true,
                name_pattern,
            ));
        }
    }

    signals
}

/// List signals in a waveform hierarchy with optional filtering.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `name_pattern` - Optional case-insensitive substring filter for signal names
/// * `hierarchy_prefix` - Optional hierarchy path prefix to filter signals (must match a scope)
/// * `recursive` - If true, list all signals recursively; if false, only list signals at the specified level
/// * `limit` - Optional maximum number of signals to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of signal paths.
pub fn list_signals(
    hierarchy: &wellen::Hierarchy,
    name_pattern: Option<&str>,
    hierarchy_prefix: Option<&str>,
    recursive: bool,
    limit: Option<isize>,
) -> Vec<String> {
    let mut signals = Vec::new();

    if let Some(prefix) = hierarchy_prefix {
        // Find the scope by path
        if let Some(scope_ref) = find_scope_by_path(hierarchy, prefix) {
            // Collect signals from this scope (and children if recursive)
            signals = collect_signals_from_scope(hierarchy, scope_ref, recursive, name_pattern);
        }
        // If scope not found, return empty signals
    } else {
        // No hierarchy prefix - start from top-level scopes
        for scope_ref in hierarchy.scopes() {
            signals.extend(collect_signals_from_scope(
                hierarchy,
                scope_ref,
                recursive,
                name_pattern,
            ));
        }
    }

    // Apply limit if provided and not -1 (unlimited)
    if let Some(limit) = limit {
        if limit >= 0 {
            signals.truncate(limit as usize);
        }
    }

    signals
}

/// Read signal values at specific time indices.
///
/// # Arguments
/// * `waveform` - The waveform to read from (must have signal loaded)
/// * `signal_ref` - The signal reference to read
/// * `time_indices` - The time indices to read values at
///
/// # Returns
/// A vector of formatted signal value strings, or an error if the operation fails.
pub fn read_signal_values(
    waveform: &wellen::simple::Waveform,
    signal_ref: wellen::SignalRef,
    time_indices: &[usize],
) -> Result<Vec<String>, String> {
    let time_table = waveform.time_table();
    let timescale = waveform.hierarchy().timescale();

    let signal = waveform
        .get_signal(signal_ref)
        .ok_or("Signal not found after loading")?;

    let mut results = Vec::new();

    for time_idx in time_indices {
        if *time_idx >= time_table.len() {
            results.push(format!(
                "Time index {} out of range (max: {})",
                time_idx,
                time_table.len() - 1
            ));
            continue;
        }

        let time_value = time_table[*time_idx];
        let formatted_time = format_time(time_value, timescale.as_ref());

        let time_table_idx: wellen::TimeTableIdx = (*time_idx)
            .try_into()
            .map_err(|_| format!("Time index {} exceeds maximum value", time_idx))?;

        let offset = signal
            .get_offset(time_table_idx)
            .ok_or("No data available for this time index")?;

        let signal_value = signal.get_value_at(&offset, 0);
        let value_str = format_signal_value(signal_value);

        results.push(format!(
            "Time index {} ({}): {}",
            time_idx, formatted_time, value_str
        ));
    }

    Ok(results)
}

/// Get metadata about a signal.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy
/// * `signal_path` - The hierarchical path to the signal
///
/// # Returns
/// A formatted string containing signal metadata, or an error if the signal is not found.
pub fn get_signal_metadata(
    hierarchy: &wellen::Hierarchy,
    signal_path: &str,
) -> Result<String, String> {
    // Find VarRef from path
    let parts: Vec<&str> = signal_path.split('.').collect();
    let var_ref = if parts.len() > 1 {
        let path = &parts[..parts.len() - 1];
        let name = parts[parts.len() - 1];
        hierarchy
            .lookup_var(path, name)
            .ok_or_else(|| format!("Signal not found: {}", signal_path))?
    } else {
        hierarchy
            .lookup_var(&[], signal_path)
            .ok_or_else(|| format!("Signal not found: {}", signal_path))?
    };

    let var = &hierarchy[var_ref];

    let width_info = match var.length() {
        Some(len) => format!("{} bits", len),
        None => "variable length (string/real)".to_string(),
    };

    let index_info = match var.index() {
        Some(idx) => format!("[{}:{}]", idx.msb(), idx.lsb()),
        None => "N/A".to_string(),
    };

    let info = format!(
        "Signal: {}\nType: {:?}\nWidth: {}\nIndex: {}",
        signal_path,
        var.var_type(),
        width_info,
        index_info
    );

    Ok(info)
}

/// Find events (changes) of a signal within a time range.
///
/// # Arguments
/// * `waveform` - The waveform to read from (must have signal loaded)
/// * `signal_ref` - The signal reference to analyze
/// * `start_idx` - Starting time index (inclusive)
/// * `end_idx` - Ending time index (inclusive)
/// * `limit` - Maximum number of events to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of formatted event strings, or an error if the operation fails.
pub fn find_signal_events(
    waveform: &wellen::simple::Waveform,
    signal_ref: wellen::SignalRef,
    start_idx: usize,
    end_idx: usize,
    limit: isize,
) -> Result<Vec<String>, String> {
    let time_table = waveform.time_table();
    let timescale = waveform.hierarchy().timescale();

    let signal = waveform
        .get_signal(signal_ref)
        .ok_or("Signal not found after loading")?;

    let mut events = Vec::new();

    for (time_idx, signal_value) in signal.iter_changes() {
        let time_idx = time_idx as usize;

        // Check if within time range
        if time_idx < start_idx || time_idx > end_idx {
            continue;
        }

        // Check limit (unless unlimited with -1)
        if limit >= 0 && events.len() >= limit as usize {
            break;
        }

        let time_value = time_table[time_idx];
        let formatted_time = format_time(time_value, timescale.as_ref());
        let value_str = format_signal_value(signal_value);

        events.push(format!(
            "Time index {} ({}): {}",
            time_idx, formatted_time, value_str
        ));
    }

    Ok(events)
}
